use bytes::BufMut;
use futures_util::StreamExt;
use warp::Filter;

#[derive(Debug)]
struct EncodeError;

#[derive(Debug)]
struct DecodeError;

#[derive(Debug)]
struct NoInput;

impl warp::reject::Reject for EncodeError {}
impl warp::reject::Reject for DecodeError {}
impl warp::reject::Reject for NoInput {}

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

fn parse_dimensions<S>(dims: S) -> Option<(u32, u32)>
where
    S: AsRef<str>,
{
    let dims = dims.as_ref().trim().to_lowercase();

    if dims == "scribe2025" {
        // Kindle Scribe 3, Scribe Colorsoft
        Some((1980, 2640))
    } else if dims == "scribe" {
        // Kindle Scribe, Scribe 2024
        Some((1860, 2480))
    } else if dims == "forma" || dims == "sage" {
        // Kobo Forma, Sage
        Some((1440, 1920))
    } else if dims == "auraone" || dims == "ellipsa" {
        // Kobo Aura One, Ellipsa (all variants)
        Some((1404, 1872))
    } else if dims == "libra"
        || dims == "oasis2018"
        || dims == "paperwhite2024"
        || dims == "colorsoft"
    {
        // Kobo Libra (all variants)
        // Kindle Colorsoft (incl. signature ed.), Oasis 2, Oasis 3, Paperwhite 6 (incl. signature ed.)
        Some((1264, 1680))
    } else if dims == "paperwhite2021" {
        // Kindle Paperwhite 5 (incl. signature ed.)
        Some((1236, 1648))
    } else if dims == "aurah2o" || dims == "aurahd" {
        // Kobo Aura H2O, Aura H2O Edition 2, Aura HD
        Some((1080, 1440))
    } else if dims == "clara"
        || dims == "glohd"
        || dims == "voyage"
        || dims == "paperwhite2015"
        || dims == "oasis"
        || dims == "kindle2022"
    {
        // Kobo Clara (all variants), Glo HD
        // Kindle 11, Kindle 2024, Oasis, Paperwhite 3, Paperwhite 4, Voyage
        Some((1072, 1448))
    } else if dims == "kindledx" {
        // Kindle DX
        Some((824, 1200))
    } else if dims == "aura" || dims == "glo" {
        // Kobo Aura, Aura Edition 2, Glo
        Some((768, 1024))
    } else if dims == "nia" || dims == "paperwhite" {
        // Kobo Nia
        // Kindle Paperwhite, Paperwhite 2
        Some((758, 1024))
    } else if dims == "kobo" || dims == "kindle" {
        // Kobo Original, Mini, Touch, Touch 2.0, WiFi
        // Kindle 1, 2, 4, 5, 7, 8, 10, Keyboard, Touch
        Some((600, 800))
    } else if dims.is_empty() {
        None
    } else {
        // TODO: Try to parse a WxH dimension
        let dims_regex =
            regex::Regex::new(r"^([0-9]+)\s?x\s?([0-9]+)$").expect("regex should always be valid");
        if let Some(caps) = dims_regex.captures(&dims) {
            let (_, [width, height]) = caps.extract();
            match (width.parse(), height.parse()) {
                (Ok(w), Ok(h)) => Some((w, h)),
                _ => None,
            }
        } else {
            eprintln!(
                "WARN: '{}' was not recognized as a valid WxH dimension string or preset",
                dims
            );
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
        eprintln!(
            "WARN: '{}' was not recognized as a valid encode strategy",
            strat
        );
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
        eprintln!(
            "WARN: '{}' was not recognized as a valid double-page strategy",
            strat
        );
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
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
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
            .map(|(w, h)| format!("{} x {}", w, h))
            .unwrap_or("None (no resize)".into())
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
        let in_width = img.width() as f32;
        let in_height = img.height() as f32;

        let mut fit_width = fit_width as f32;
        let fit_height = fit_height as f32;

        let aspect_ratio = in_width / in_height;

        if (config.double_page_strategy == DoublePageStrategy::IfMuchWider && aspect_ratio > 1.5)
            || (config.double_page_strategy == DoublePageStrategy::IfWider && aspect_ratio > 1.0)
        {
            fit_width *= 2.0;
            eprintln!(
                "Fitting {}x{} to {}x{} (double page)",
                in_width, in_height, fit_width, fit_height
            );
        } else {
            eprintln!(
                "Fitting {}x{} to {}x{}",
                in_width, in_height, fit_width, fit_height
            );
        }

        let scale_factor = f32::min(fit_width / in_width, fit_height / in_height);

        if scale_factor >= 1.0 {
            return img;
        }

        let out_width = (in_width * scale_factor).ceil() as u32;
        let out_height = (in_height * scale_factor).ceil() as u32;

        img.resize(out_width, out_height, filter)
    } else {
        img
    }
}

/// Encode the image to WebP, returning whichever is the smaller of an 85 quality lossy
/// encode or a lossless encode.
fn webp_encode(img: image::DynamicImage, config: &Config) -> Option<Vec<u8>> {
    let encoder = webp::Encoder::from_image(&img).ok()?;

    Some(Vec::from(&*match config.encode_strategy {
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
        .expect("failed to listen for shutdown signal")
}

async fn handle_errors(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    let code;
    let message;

    if let Some(EncodeError) = err.find() {
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "Got valid input, but failed to encode";
    } else if let Some(DecodeError) = err.find() {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = "Invalid input image";
    } else if let Some(NoInput) = err.find() {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = "`image` field missing from request"
    } else {
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "Unknown error";
    }

    let timestamp: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
    eprintln!(
        "[{}] Failed request with code {}: {}",
        timestamp.format("%Y-%m-%dT%H:%M:%S"),
        code,
        message
    );

    Ok::<warp::reply::WithStatus<&str>, _>(warp::reply::with_status(message, code))
}

#[tokio::main]
async fn main() {
    let config: &'static Config = Box::leak(Box::new(load_config()));

    log_config(config);

    let convert = warp::multipart::form()
        .and_then(move |mut form: warp::multipart::FormData| async move {
            while let Some(Ok(mut field)) = form.next().await {
                if field.name() == "image" {
                    let mut img_bytes: Vec<u8> = Vec::new();

                    while let Some(Ok(content)) = field.data().await {
                        img_bytes.put(content);
                    }

                    if let Ok(img) = image::ImageReader::new(std::io::Cursor::new(img_bytes))
                        .with_guessed_format()
                        && let Ok(img) = img.decode()
                    {
                        return webp_encode(
                            scale_to_fit(img, image::imageops::FilterType::CatmullRom, config),
                            config,
                        )
                        .ok_or(warp::reject::custom(EncodeError));
                    }

                    return Err(warp::reject::custom(DecodeError));
                }
            }

            Err(warp::reject::custom(NoInput))
        })
        .recover(handle_errors);

    warp::serve(convert)
        .bind(config.listen_on)
        .await
        .graceful(stop_signal())
        .run()
        .await;
}
