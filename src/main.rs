use bytes::BufMut;
use ereadifier::config::{Config, DoublePageStrategy, EncodeStrategy, load_config, log_config};
use futures_util::StreamExt;
use serde::Serialize;
use warp::Filter;

#[derive(Debug)]
struct EncodeError(String);

#[derive(Debug)]
struct DecodeError(String);

#[derive(Debug)]
struct NoInput;

impl warp::reject::Reject for EncodeError {}
impl warp::reject::Reject for DecodeError {}
impl warp::reject::Reject for NoInput {}

#[derive(Serialize)]
struct ErrorDetails {
    code: u16,
    message: String,
}

/// Downscale the image to fit within the target dimensions while preserving aspect ratio.
/// Depending on double page strategy, the target dimensions may be considered twice as
/// wide if the image is detected to be a double page (a page that is wider than 3:2 aspect
/// if using the "double page is much wider" strategy, or wider than 1:1 with the "wider"
/// strategy). If the image already fits or no dimensions are provided, it is returned as-is,
/// otherwise it is scaled using the chosen filter.
fn scale_to_fit(
    img: image::DynamicImage,
    filter: image::imageops::FilterType,
    config: &Config,
) -> image::DynamicImage {
    if let Some((fit_width, fit_height)) = config.dimensions {
        let in_width = f64::from(img.width());
        let in_height = f64::from(img.height());

        let mut fit_width = f64::from(fit_width);
        let fit_height = f64::from(fit_height);

        let aspect_ratio = in_width / in_height;

        if (config.double_page_strategy == DoublePageStrategy::IfMuchWider && aspect_ratio > 1.5)
            || (config.double_page_strategy == DoublePageStrategy::IfWider && aspect_ratio > 1.0)
        {
            fit_width *= 2.0;
            if config.debug_mode {
                eprintln!(
                    "Fitting {in_width}x{in_height} to {fit_width}x{fit_height} (double page)"
                );
            }
        } else if config.debug_mode {
            eprintln!("Fitting {in_width}x{in_height} to {fit_width}x{fit_height}");
        }

        let scale_factor = f64::min(fit_width / in_width, fit_height / in_height);

        if scale_factor >= 1.0 {
            return img;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let out_width = (in_width * scale_factor).ceil() as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let out_height = (in_height * scale_factor).ceil() as u32;

        img.resize(out_width, out_height, filter)
    } else {
        img
    }
}

/// As the WebP encoder does not support images that are not RGB8 or RGBA8, we convert into
/// one of those types based on whether the current type has an alpha channel
fn ensure_rgb(img: &image::DynamicImage) -> image::DynamicImage {
    if img.has_alpha() {
        image::DynamicImage::ImageRgba8(img.to_rgba8())
    } else {
        image::DynamicImage::ImageRgb8(img.to_rgb8())
    }
}

/// Encode the image to WebP, returning whichever is the smaller of an 85 quality lossy
/// encode or a lossless encode.
fn webp_encode<'a>(img: &'a image::DynamicImage, config: &Config) -> Result<Vec<u8>, &'a str> {
    let encoder = webp::Encoder::from_image(img)?;

    Ok(Vec::from(&*match config.encode_strategy {
        EncodeStrategy::Smallest => {
            let lossy_out = encoder.encode(config.lossy_quality);
            let lossless_out = encoder.encode_lossless();

            if lossy_out.len() < lossless_out.len() {
                lossy_out
            } else {
                lossless_out
            }
        }
        EncodeStrategy::Lossless => encoder.encode_lossless(),
        EncodeStrategy::Lossy => encoder.encode(config.lossy_quality),
    }))
}

/// Listen for SIGINT (Ctrl+C) or SIGTERM on Unix, Ctrl+C on any other platform
async fn stop_signal() {
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to register SIGTERM handler");

    #[cfg(unix)]
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {  },
        _ = sigterm.recv() => { },
    }

    #[cfg(not(unix))]
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for shutdown signal");
}

async fn handle_errors(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    let code;
    let message;

    if let Some(EncodeError(msg)) = err.find() {
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = format!("Got valid input, but failed to encode: {msg}");
    } else if let Some(DecodeError(msg)) = err.find() {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = format!("Failed to decode: {msg}");
    } else if let Some(NoInput) = err.find() {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = "`image` field missing from request".into();
    } else {
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = format!("Unhandled error: {err:?}");
    }

    let timestamp: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
    eprintln!(
        "[{}] Failed request with code {}: {}",
        timestamp.format("%Y-%m-%dT%H:%M:%S"),
        code,
        message
    );

    let json = warp::reply::json(&ErrorDetails {
        code: code.as_u16(),
        message,
    });

    Ok(warp::reply::with_status(json, code))
}

async fn handle_convert(
    mut form: warp::multipart::FormData,
    config: &'static Config,
) -> Result<Vec<u8>, warp::Rejection> {
    while let Some(Ok(mut field)) = form.next().await {
        if field.name() != "image" {
            continue;
        }

        let mut img_bytes: Vec<u8> = Vec::new();

        while let Some(Ok(content)) = field.data().await {
            img_bytes.put(content);
        }

        let img = image::ImageReader::new(std::io::Cursor::new(img_bytes))
            .with_guessed_format()
            .map_err(|e| warp::reject::custom(DecodeError(e.to_string())))?
            .decode()
            .map_err(|e| warp::reject::custom(DecodeError(e.to_string())))?;

        let img = ensure_rgb(&scale_to_fit(
            img,
            image::imageops::FilterType::CatmullRom,
            config,
        ));

        return webp_encode(&img, config)
            .map_err(|msg| warp::reject::custom(EncodeError(msg.into())));
    }

    Err(warp::reject::custom(NoInput))
}

#[tokio::main]
async fn main() {
    let config: &'static Config = Box::leak(Box::new(load_config()));

    log_config(config);

    let convert = warp::multipart::form()
        .max_length(Some(10 * 1024 * 1024))
        .map(move |form| (form, config))
        .untuple_one()
        .and_then(handle_convert)
        .recover(handle_errors);

    let health_check = warp::path("health").map(|| "OK");

    warp::serve(health_check.or(convert))
        .bind(config.listen_on)
        .await
        .graceful(stop_signal())
        .run()
        .await;
}
