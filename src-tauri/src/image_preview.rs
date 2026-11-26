use axum::{
    body::Body,
    http::{header, StatusCode},
    response::Response,
};
use image::{ImageDecoder, ImageFormat, ImageReader, Limits};
use std::{
    io::Cursor,
    path::Path,
    sync::{Arc, LazyLock},
};
use tokio::{io::AsyncReadExt, sync::Semaphore};

pub const MAX_INPUT: u64 = 16 * 1024 * 1024;
static SLOTS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(2)));

fn dimensions(width: u32, height: u32) -> Result<(), StatusCode> {
    if width > 8192 || height > 8192 || u64::from(width) * u64::from(height) > 16_000_000 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    Ok(())
}

fn convert(bytes: Vec<u8>) -> Result<Vec<u8>, StatusCode> {
    if bytes.len() as u64 > MAX_INPUT {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Tiff);
    let mut limits = Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoder = reader.into_decoder().map_err(image_error)?;
    let (w, h) = decoder.dimensions();
    dimensions(w, h)?;
    let image = image::DynamicImage::from_decoder(decoder).map_err(image_error)?;
    // Standard 8-bit RGBA PNG is consistently supported by web browsers.
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image.to_rgba8())
        .write_to(&mut output, ImageFormat::Png)
        .map_err(image_error)?;
    Ok(output.into_inner())
}

fn image_error(error: image::ImageError) -> StatusCode {
    if matches!(error, image::ImageError::Limits(_)) {
        StatusCode::PAYLOAD_TOO_LARGE
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    }
}

pub async fn respond(path: &Path, head: bool) -> Response {
    respond_with_slots(path, head, SLOTS.clone()).await
}

async fn respond_with_slots(path: &Path, head: bool, slots: Arc<Semaphore>) -> Response {
    let result = async {
        let permit = slots
            .try_acquire_owned()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let file = tokio::fs::File::open(path)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
        let mut bytes = Vec::new();
        file.take(MAX_INPUT + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        if bytes.len() as u64 > MAX_INPUT {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        // Keep the permit inside the blocking worker, including if the request is cancelled.
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            convert(bytes)
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    }
    .await;
    let (status, bytes, content_type) = match result {
        Ok(bytes) => (StatusCode::OK, bytes, "image/png"),
        Err(status) => {
            let message = match status {
                StatusCode::PAYLOAD_TOO_LARGE => {
                    "TIFF 超出预览大小或像素限制，请使用 ?download=1 下载原图"
                }
                StatusCode::SERVICE_UNAVAILABLE => "图片转换繁忙，请稍后重试",
                _ => "TIFF 无法预览，请使用 ?download=1 下载原图",
            };
            (
                status,
                message.as_bytes().to_vec(),
                "text/plain; charset=utf-8",
            )
        }
    };
    let length = bytes.len();
    let mut response = Response::new(if head {
        Body::empty()
    } else {
        Body::from(bytes)
    });
    *response.status_mut() = status;
    for (key, value) in [
        (header::CONTENT_TYPE, content_type),
        (header::CACHE_CONTROL, "no-store"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ] {
        response.headers_mut().insert(key, value.parse().unwrap());
    }
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, length.into());
    if status == StatusCode::SERVICE_UNAVAILABLE {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, "1".parse().unwrap());
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiff_converts_to_png_and_bad_data_fails() {
        let mut tiff = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(3, 2)
            .write_to(&mut tiff, image::ImageFormat::Tiff)
            .unwrap();
        let png = convert(tiff.into_inner()).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (3, 2));
        assert_eq!(
            convert(b"not tiff".to_vec()).unwrap_err(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn dimensions_and_input_are_bounded() {
        assert!(dimensions(8192, 1).is_ok());
        assert!(dimensions(8193, 1).is_err());
        assert!(dimensions(5000, 5000).is_err());
        assert_eq!(
            convert(vec![0; MAX_INPUT as usize + 1]).unwrap_err(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[tokio::test]
    async fn busy_converter_rejects_without_waiting() {
        // A local semaphore avoids racing other image tests for the global slots.
        let slots = Arc::new(Semaphore::new(2));
        let _first = slots.clone().try_acquire_owned().unwrap();
        let _second = slots.clone().try_acquire_owned().unwrap();
        let response = respond_with_slots(Path::new("does-not-exist.tiff"), false, slots).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()[header::RETRY_AFTER], "1");
    }
}
