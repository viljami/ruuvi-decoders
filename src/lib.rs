//! Ruuvi BLE Advertisement Decoders
//!
//! This crate provides decoders for Ruuvi sensor BLE advertisements supporting:
//! - Data Format 5 (`RAWv2`)
//! - Data Format 6 (`RAWv3`)
//! - Data Format E1 (Encrypted)
//!
//! # Example
//!
//! ```rust
//! use ruuvi_decoders::{decode, RuuviData};
//!
//! let hex_data = "1BFFFF04990512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
//! let decoded = decode(hex_data).unwrap();
//!
//! match decoded {
//!     RuuviData::V5(data) => {
//!         println!("Temperature: {:?}°C", data.temperature);
//!         println!("Humidity: {:?}%", data.humidity);
//!     },
//!     _ => println!("Other format"),
//! }
//! ```

pub mod air_quality;
pub mod e1;
pub mod error;
pub mod ruuvi_data;
pub mod v5;
pub mod v6;

pub use error::{DecodeError, Result};
pub use ruuvi_data::{DataFormat, RuuviData};

pub const MANUFACTURER_ID_LENGTH: usize = 2;

/// Main entry point for decoding Ruuvi BLE advertisement data
///
/// # Arguments
///
/// * `hex_data` - Hex string of the Ruuvi payload (without the 9904 manufacturer prefix)
///
/// # Returns
///
/// * `Ok(RuuviData)` - Successfully decoded data
/// * `Err(DecodeError)` - Decoding failed
///
/// # Example
///
/// ```rust
/// use ruuvi_decoders::decode;
///
/// let hex_data = "1BFFFF04990512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
/// let result = decode(hex_data).unwrap();
/// ```
///
/// # Errors
///
/// * `DecodeError::InvalidHex` - Invalid hex string
/// * `DecodeError::InvalidLength` - Invalid length of hex string
/// * `DecodeError::UnsupportedFormat` - Unsupported data format
pub fn decode(ble_data: &str) -> Result<RuuviData> {
    let clean_data = ble_data
        .trim()
        .trim_start_matches("0x")
        .replace(' ', "")
        .to_uppercase();

    if clean_data.is_empty() {
        return Err(DecodeError::NoData);
    }

    if !clean_data.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(DecodeError::InvalidData(format!(
            "Contains non hex characters: {clean_data}"
        )));
    }

    let payload_start = validate_ruuvi_manufacturer_id(ble_data)?;
    let payload_start = payload_start / 2; // to byte index
    // Convert hex to bytes
    let bytes = hex_to_bytes(&clean_data)?;

    if bytes.is_empty() {
        return Err(DecodeError::InvalidData(format!(
            "No bytes provided in hex_string: {clean_data}"
        )));
    }

    // Do not calculate length from bytes, use payload_start instead for now
    // let Some(length) = bytes.get(0) else {
    //     return Err(DecodeError::InvalidLength(
    //         "Got bytes, but length byte is missing".into(),
    //     ));
    // };

    // bytes[1..3] = Manufacturer Specified Data
    // bytes[3..5] = Ruuvi Manufacturer ID
    let bytes = &bytes[payload_start..];

    // Determine data format from first byte
    match bytes[0] {
        5 => {
            let data = v5::decode(bytes)?;
            Ok(RuuviData::V5(data))
        }
        6 => {
            let data = v6::decode(bytes)?;
            Ok(RuuviData::V6(data))
        }
        0xE1 => {
            let data = e1::decode(bytes)?;
            Ok(RuuviData::E1(data))
        }
        format => Err(DecodeError::UnsupportedFormat(format)),
    }
}

/// Extract Ruuvi data from a full BLE advertisement
///
/// Looks for the Ruuvi manufacturer data (0x9904) and extracts the payload
///
/// # Errors
///
/// * `DecodeError::InvalidLength` - Provided length does not match the expected payload length
/// * `DecodeError::UnsupportedFormat` - Unsupported data format
///
/// # Arguments
///
/// * `ble_data` - Full BLE advertisement hex string
///
/// # Returns
///
/// * `Some(String)` - Extracted Ruuvi payload hex
/// * `None` - No Ruuvi data found
pub fn validate_ruuvi_manufacturer_id(ble_data: &str) -> Result<usize> {
    // Look for Ruuvi manufacturer ID (0x9904 in little-endian format in BLE ads)
    // The actual pattern in BLE advertisements could be "9904" or "0499" depending on endianness
    for pattern in ["9904", "0499"] {
        if let Some(start_idx) = ble_data.find(pattern) {
            let payload_start = start_idx + 4; // Skip the 4-char manufacturer ID

            // Extract payload - length depends on format, but we'll try to get a reasonable amount
            // Data Format 5 should be 24 bytes = 48 hex chars
            if payload_start <= 10 {
                return Ok(payload_start);
            }
        }
    }

    Err(DecodeError::MissingManufacturerId)
}

/// Convert hex string to bytes
fn hex_to_bytes(hex_str: &str) -> Result<Vec<u8>> {
    if !hex_str.len().is_multiple_of(2) {
        return Err(DecodeError::InvalidHex(format!(
            "Odd number of hex characters: {}",
            hex_str.len()
        )));
    }

    hex::decode(hex_str).map_err(|_| DecodeError::InvalidHex(hex_str.to_string()))
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;

    #[test]
    fn test_decode_ble() {
        let ble_data = "2BFFFF9904E112622998C8B300050008000A000A02312C00FFFFFFFFFFFFAEEB38F8FFFFFFFFFFD83FFFFF2A03030398FC";
        let result = decode(ble_data);
        let data = result.expect("Failed to decode Ruuvi data");
        assert_debug_snapshot!(data);
    }

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(hex_to_bytes("01FF").unwrap(), vec![0x01, 0xFF]);
        assert_eq!(hex_to_bytes("").unwrap(), Vec::<u8>::new());
        assert!(hex_to_bytes("0").is_err()); // Odd length
        assert!(hex_to_bytes("GG").is_err()); // Invalid hex
    }

    #[test]
    fn test_validate_ruuvi_ble_data() {
        let ble_data = "99040512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
        validate_ruuvi_manufacturer_id(ble_data).expect("Manufacturer ID not found 9904");
        let ble_data_le = "04990512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
        validate_ruuvi_manufacturer_id(ble_data_le).expect("Manufacturer ID not found 0499");
        // Test with no Ruuvi data
        let non_ruuvi = "020106030316910255AA";
        assert_eq!(
            validate_ruuvi_manufacturer_id(non_ruuvi),
            Err(DecodeError::MissingManufacturerId)
        );
    }

    #[test]
    fn test_decode_empty_data() {
        assert!(decode("").is_err());
    }

    #[test]
    fn test_unsupported_format() {
        // Format 99 doesn't exist
        let result = decode("1BFFFF049963000000000000000000000000000000000000000000000000");

        assert_eq!(result, Err(DecodeError::UnsupportedFormat(99)));
    }

    #[test]
    fn test_decoding_ruuvi_data() {
        let ble_data = "18FFFF99040512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
        let payload = decode(ble_data).expect("Failed to extract Ruuvi data");
        match payload {
            RuuviData::V5(_) => (),
            _ => panic!("Unexpected data format"),
        }

        let ble_data = "14FFFF990406170C5668C79E007000C90501D9FFCD004C884F";
        let payload = decode(ble_data).expect("Failed to extract Ruuvi data");
        match payload {
            RuuviData::V6(_) => (),
            _ => panic!("Unexpected data format"),
        }

        let ble_data = "2BFFFF9904E1170C5668C79E0065007004BD11CA00C90A0213E0AC000000DECDEE100000000000CBB8334C884F";
        let payload = decode(ble_data).expect("Failed to extract Ruuvi data");
        match payload {
            RuuviData::E1(_) => (),
            _ => panic!("Unexpected data format"),
        }
    }
}
