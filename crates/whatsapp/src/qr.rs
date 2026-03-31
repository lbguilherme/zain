use chromium_driver::dom::{Dom, Element};

use crate::error::{Result, WhatsappError};

/// CSS selectors for the QR code canvas element.
pub(crate) const QR_SELECTORS: &[&str] = &[
    r#"div[data-testid="qrcode"] canvas"#,
    r#"canvas[aria-label="Scan this QR code to link a device!"]"#,
    r#"div[data-ref] canvas"#,
];

/// Tries to find the QR canvas using multiple selectors.
pub(crate) async fn find_qr_element(dom: &Dom) -> chromium_driver::Result<Option<Element>> {
    for selector in QR_SELECTORS {
        if let Some(el) = dom.try_query_selector(selector).await? {
            return Ok(Some(el));
        }
    }
    Ok(None)
}

/// Extracts the QR code string from a canvas element: screenshot + decode.
pub(crate) async fn extract_from_element(el: &Element) -> Result<String> {
    let png = el.screenshot_png().await?;
    decode_qr(&png)
}

/// Decodes a QR code from PNG image bytes using rqrr.
fn decode_qr(png_bytes: &[u8]) -> Result<String> {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().map_err(|e| WhatsappError::QrCodeDecode(e.to_string()))?;
    let buf_size = reader.output_buffer_size()
        .ok_or_else(|| WhatsappError::QrCodeDecode("unknown PNG output size".into()))?;
    let mut buf = vec![0u8; buf_size];
    let info = reader.next_frame(&mut buf).map_err(|e| WhatsappError::QrCodeDecode(e.to_string()))?;
    let width = info.width as usize;
    let height = info.height as usize;

    // Convert to luma (grayscale) depending on color type.
    let luma: Vec<u8> = match info.color_type {
        png::ColorType::Grayscale => buf[..width * height].to_vec(),
        png::ColorType::GrayscaleAlpha => {
            buf[..width * height * 2]
                .chunks_exact(2)
                .map(|px| px[0])
                .collect()
        }
        png::ColorType::Rgb => {
            buf[..width * height * 3]
                .chunks_exact(3)
                .map(|px| ((px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000) as u8)
                .collect()
        }
        png::ColorType::Rgba => {
            buf[..width * height * 4]
                .chunks_exact(4)
                .map(|px| ((px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000) as u8)
                .collect()
        }
        _ => return Err(WhatsappError::QrCodeDecode("unsupported PNG color type".into())),
    };

    let mut grid = rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| {
        luma[y * width + x]
    });

    let grids = grid.detect_grids();
    let grid = grids
        .first()
        .ok_or_else(|| WhatsappError::QrCodeDecode("no QR code found".into()))?;

    let (_, content) = grid
        .decode()
        .map_err(|e| WhatsappError::QrCodeDecode(e.to_string()))?;

    Ok(content)
}
