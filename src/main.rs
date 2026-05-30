use std::collections::HashMap;

use bytes::BufMut;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
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

struct Config {
    dimensions: Option<(u32, u32)>,
    lossy_quality: f32,
    encode_strategy: EncodeStrategy,
    double_page_strategy: DoublePageStrategy,
    listen_on: std::net::SocketAddr,
}

enum EncodeStrategy {
    Smallest,
    Lossless,
    Lossy,
}

#[derive(PartialEq)]
enum DoublePageStrategy {
    Ignore,
    IfWider,
    IfMuchWider,
}

impl std::fmt::Display for EncodeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match *self {
            Self::Smallest => "smallest (Compare lossless and lossy, use smallest)",
            Self::Lossless => "lossless (Always use lossless)",
            Self::Lossy => "lossy (Always use lossy)",
        })
    }
}

impl std::fmt::Display for DoublePageStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match *self {
            Self::Ignore => "ignore (No special handling of double pages)",
            Self::IfWider => "wider (Assume input is a double page if wider than 1:1)",
            Self::IfMuchWider => "much_wider (Assume input is a double page if wider than 3:2)",
        })
    }
}

const PRESETS: &str = include_str!("../presets.toml");

#[derive(Deserialize)]
struct Preset {
    width: u32,
    height: u32,
    devices: Vec<String>,
}

fn parse_dimensions<S>(dims: S) -> Option<(u32, u32)>
where
    S: AsRef<str>,
{
    let dims = dims.as_ref().trim().to_lowercase();

    let dims_regex =
        regex::Regex::new(r"^([0-9]+)\s?x\s?([0-9]+)$").expect("regex should always be valid");
    if let Some(caps) = dims_regex.captures(&dims) {
        let (_, [width, height]) = caps.extract();
        match (width.parse(), height.parse()) {
            (Ok(w), Ok(h)) => Some((w, h)),
            _ => None,
        }
    } else {
        let presets: HashMap<String, Preset> =
            toml::from_str(PRESETS).expect("presets.toml must always contain valid data");

        if let Some(preset) = presets.get(&dims) {
            eprintln!("Using preset `{dims}`, which supports:");
            for device in &preset.devices {
                eprintln!("\t- {device}");
            }
            Some((preset.width, preset.height))
        } else {
            eprintln!(
                "WARN: '{dims}' was not recognized as a valid WxH dimension string or preset."
            );
            eprintln!("Valid presets:");
            for (preset_name, Preset { width, height, .. }) in &presets {
                eprintln!("\t- {preset_name} ({width}x{height})");
            }
            None
        }
    }
}

fn parse_encode_strategy<S>(strat: S) -> Option<EncodeStrategy>
where
    S: AsRef<str>,
{
    let strat = strat.as_ref().trim().to_lowercase();

    if strat == "smallest" {
        Some(EncodeStrategy::Smallest)
    } else if strat == "lossy" {
        Some(EncodeStrategy::Lossy)
    } else if strat == "lossless" {
        Some(EncodeStrategy::Lossless)
    } else if strat.is_empty() {
        None
    } else {
        eprintln!("WARN: '{strat}' was not recognized as a valid encode strategy");
        None
    }
}

fn parse_double_page_strategy<S>(strat: S) -> Option<DoublePageStrategy>
where
    S: AsRef<str>,
{
    let strat = strat.as_ref().trim().to_lowercase();

    if strat == "much_wider" {
        Some(DoublePageStrategy::IfMuchWider)
    } else if strat == "wider" {
        Some(DoublePageStrategy::IfWider)
    } else if strat == "ignore" {
        Some(DoublePageStrategy::Ignore)
    } else if strat.is_empty() {
        None
    } else {
        eprintln!("WARN: '{strat}' was not recognized as a valid double-page strategy");
        None
    }
}

fn load_config() -> Config {
    Config {
        dimensions: std::env::var("EREADIFIER_DIMENSIONS")
            .ok()
            .and_then(parse_dimensions),
        encode_strategy: std::env::var("EREADIFIER_ENCODE")
            .ok()
            .and_then(parse_encode_strategy)
            .unwrap_or(EncodeStrategy::Lossless),
        double_page_strategy: std::env::var("EREADIFIER_DOUBLE_PAGE")
            .ok()
            .and_then(parse_double_page_strategy)
            .unwrap_or(DoublePageStrategy::IfMuchWider),
        lossy_quality: std::env::var("EREADIFIER_LOSSY_QUALITY")
            .ok()
            .and_then(|q| q.parse::<f32>().ok())
            .unwrap_or(85.0)
            .clamp(0.0, 100.0),
        listen_on: std::env::var("EREADIFIER_LISTEN")
            .ok()
            .and_then(|addr| addr.parse::<std::net::SocketAddr>().ok())
            .unwrap_or(std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                80,
            )),
    }
}

fn log_config(config: &Config) {
    eprintln!("Listening on {} with config:", config.listen_on);
    eprintln!(
        "\tDimensions: {}",
        config
            .dimensions
            .map_or("None (no resize)".into(), |(w, h)| format!("{w} x {h}"))
    );
    eprintln!("\tLossy quality target (0~100): {}", config.lossy_quality);
    eprintln!("\tEncoding strategy: {}", config.encode_strategy);
    eprintln!("\tDouble page strategy: {}", config.double_page_strategy);
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
            eprintln!("Fitting {in_width}x{in_height} to {fit_width}x{fit_height} (double page)");
        } else {
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

#[tokio::main]
async fn main() {
    let config: &'static Config = Box::leak(Box::new(load_config()));

    log_config(config);

    let convert = warp::multipart::form()
        .max_length(Some(10 * 1024 * 1024))
        .and_then(move |mut form: warp::multipart::FormData| async move {
            while let Some(Ok(mut field)) = form.next().await {
                if field.name() == "image" {
                    let mut img_bytes: Vec<u8> = Vec::new();

                    while let Some(Ok(content)) = field.data().await {
                        img_bytes.put(content);
                    }

                    return match image::ImageReader::new(std::io::Cursor::new(img_bytes))
                        .with_guessed_format()
                    {
                        Ok(img) => match img.decode() {
                            Ok(img) => webp_encode(
                                &ensure_rgb(&scale_to_fit(
                                    img,
                                    image::imageops::FilterType::CatmullRom,
                                    config,
                                )),
                                config,
                            )
                            .map_err(|msg| warp::reject::custom(EncodeError(msg.into()))),
                            Err(e) => Err(warp::reject::custom(DecodeError(e.to_string()))),
                        },
                        Err(e) => Err(warp::reject::custom(DecodeError(e.to_string()))),
                    };
                }
            }

            Err(warp::reject::custom(NoInput))
        })
        .recover(handle_errors);

    let health_check = warp::path("health").map(|| "OK");

    warp::serve(health_check.or(convert))
        .bind(config.listen_on)
        .await
        .graceful(stop_signal())
        .run()
        .await;
}
