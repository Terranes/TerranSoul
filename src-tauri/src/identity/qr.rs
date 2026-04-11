use qrcode::{render::svg, QrCode};

use super::device::DeviceInfo;

/// Generate an SVG QR code that encodes the pairing payload for `info`.
///
/// The QR payload is a compact JSON object:
/// `{"app":"TerranSoul","v":1,"device_id":"<uuid>","pub_key":"<base64>","name":"<name>"}`
pub fn generate_pairing_qr(info: &DeviceInfo) -> Result<String, String> {
    let payload = serde_json::json!({
        "app": "TerranSoul",
        "v": 1,
        "device_id": info.device_id,
        "pub_key": info.public_key_b64,
        "name": info.name,
    });
    let data = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

    let code = QrCode::new(data.as_bytes()).map_err(|e| e.to_string())?;
    let svg_string = code
        .render::<svg::Color<'_>>()
        .min_dimensions(200, 200)
        .build();
    Ok(svg_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::device::DeviceIdentity;

    #[test]
    fn pairing_qr_produces_non_empty_svg() {
        let identity = DeviceIdentity::generate();
        let info = identity.device_info("test-device");
        let svg = generate_pairing_qr(&info).unwrap();
        assert!(!svg.is_empty());
    }

    #[test]
    fn pairing_qr_output_is_svg_format() {
        let identity = DeviceIdentity::generate();
        let info = identity.device_info("test-device");
        let svg = generate_pairing_qr(&info).unwrap();
        assert!(svg.contains("<svg") || svg.contains("<?xml"));
    }
}
