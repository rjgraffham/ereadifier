use bytes::BufMut;
use futures_util::StreamExt;
use warp::Filter;
use warp::multipart::FormData;

#[derive(Debug)]
struct EncodeError;

#[derive(Debug)]
struct DecodeError;

#[derive(Debug)]
struct NoInput;

impl warp::reject::Reject for EncodeError {}
impl warp::reject::Reject for DecodeError {}
impl warp::reject::Reject for NoInput {}

#[tokio::main]
async fn main() {
    let convert = warp::multipart::form().and_then(|mut form: FormData| async move {
        while let Some(Ok(mut field)) = form.next().await {
            if field.name() == "image" {
                let mut img_bytes: Vec<u8> = Vec::new();

                while let Some(Ok(content)) = field.data().await {
                    img_bytes.put(content);
                }

                if let Ok(img) =
                    image::ImageReader::new(std::io::Cursor::new(img_bytes)).with_guessed_format()
                    && let Ok(img) = img.decode()
                {
                    let in_width = img.width() as f32;
                    let in_height = img.height() as f32;

                    let scale_factor =
                        f32::min(1.0, f32::min(1072.0 / in_width, 1488.0 / in_height));

                    let out_width = (in_width * scale_factor).ceil() as u32;
                    let out_height = (in_height * scale_factor).ceil() as u32;

                    let resized_img = img.resize(
                        out_width,
                        out_height,
                        image::imageops::FilterType::CatmullRom,
                    );

                    let mut resized_img_bytes: Vec<u8> = Vec::new();

                    if let Ok(()) = resized_img.write_to(
                        std::io::Cursor::new(&mut resized_img_bytes),
                        image::ImageFormat::WebP,
                    ) {
                        return Ok::<_, warp::Rejection>(resized_img_bytes);
                    }

                    return Err(warp::reject::custom(EncodeError));
                }

                return Err(warp::reject::custom(DecodeError));
            }
        }

        Err(warp::reject::custom(NoInput))
    });

    warp::serve(convert).run(([0, 0, 0, 0], 80)).await;
}
