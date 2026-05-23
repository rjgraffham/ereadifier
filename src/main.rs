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

/// Downscale the image to fit within the target dimensions while preserving aspect ratio.
/// If the image already fits, it is returned as-is, otherwise it is scaled using the
/// chosen filter.
fn scale_to_fit(
    fit_width: u32,
    fit_height: i32,
    filter: image::imageops::FilterType,
    img: image::DynamicImage,
) -> image::DynamicImage {
    let in_width = img.width() as f32;
    let in_height = img.height() as f32;
    let fit_width = fit_width as f32;
    let fit_height = fit_height as f32;

    let scale_factor = f32::min(fit_width / in_width, fit_height / in_height);

    if scale_factor >= 1.0 {
        return img;
    }

    let out_width = (in_width * scale_factor).ceil() as u32;
    let out_height = (in_height * scale_factor).ceil() as u32;

    img.resize(out_width, out_height, filter)
}

/// Encode the image to WebP, returning whichever is the smaller of an 85 quality lossy
/// encode or a lossless encode.
fn webp_encode(img: image::DynamicImage) -> Option<Vec<u8>> {
    let encoder = webp::Encoder::from_image(&img).ok()?;

    let lossy_out = encoder.encode(85.0);
    let lossless_out = encoder.encode_lossless();

    Some(Vec::from(&*if lossy_out.len() < lossless_out.len() {
        lossy_out
    } else {
        lossless_out
    }))
}

#[tokio::main]
async fn main() {
    let convert =
        warp::multipart::form().and_then(|mut form: warp::multipart::FormData| async move {
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
                        return webp_encode(scale_to_fit(
                            1072,
                            1488,
                            image::imageops::FilterType::CatmullRom,
                            img,
                        ))
                        .ok_or(warp::reject::custom(EncodeError));
                    }

                    return Err(warp::reject::custom(DecodeError));
                }
            }

            Err(warp::reject::custom(NoInput))
        });

    warp::serve(convert).run(([0, 0, 0, 0], 80)).await;
}
