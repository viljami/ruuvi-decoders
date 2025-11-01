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
//! let hex_data = "0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
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
/// let hex_data = "0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
/// let result = decode(hex_data).unwrap();
/// ```
///
/// # Errors
///
/// * `DecodeError::InvalidHex` - Invalid hex string
/// * `DecodeError::InvalidLength` - Invalid length of hex string
/// * `DecodeError::UnsupportedFormat` - Unsupported data format
pub fn decode(hex_data: &str) -> Result<RuuviData> {
    // Clean up hex string - remove whitespace and 0x prefix if present
    let clean_hex = hex_data.trim().trim_start_matches("0x").replace(' ', "");

    // Convert hex to bytes
    let bytes = hex_to_bytes(&clean_hex)?;

    if bytes.is_empty() {
        return Err(DecodeError::InvalidLength("Empty data".into()));
    }

    // Determine data format from first byte
    match bytes[0] {
        5 => {
            let data = v5::decode(&bytes)?;
            Ok(RuuviData::V5(data))
        }
        6 => {
            let data = v6::decode(&bytes)?;
            Ok(RuuviData::V6(data))
        }
        0xE1 => {
            let data = e1::decode(&bytes)?;
            Ok(RuuviData::E1(data))
        }
        format => Err(DecodeError::UnsupportedFormat(format)),
    }
}

/// Extract Ruuvi data from a full BLE advertisement
///
/// Looks for the Ruuvi manufacturer data (0x9904) and extracts the payload
///
/// # Arguments
///
/// * `ble_data` - Full BLE advertisement hex string
///
/// # Returns
///
/// * `Some(String)` - Extracted Ruuvi payload hex
/// * `None` - No Ruuvi data found
#[must_use]
pub fn extract_ruuvi_from_ble(ble_data: &str) -> Option<String> {
    let clean_data = ble_data.trim().to_uppercase();

    // Validate hex format
    if !clean_data.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    // Look for Ruuvi manufacturer ID (0x9904 in little-endian format in BLE ads)
    // The actual pattern in BLE advertisements could be "9904" or "0499" depending on endianness
    for pattern in ["9904", "0499"] {
        if let Some(start_idx) = clean_data.find(pattern) {
            if start_idx != 0 {
                continue;
            }

            let payload_start = start_idx + 4; // Skip the 4-char manufacturer ID

            // Extract payload - length depends on format, but we'll try to get a reasonable amount
            // Data Format 5 should be 24 bytes = 48 hex chars
            if payload_start <= clean_data.len() {
                let payload = &clean_data[payload_start..];
                return Some(payload.to_string());
            }
        }
    }

    None
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
    use super::*;

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(hex_to_bytes("01FF").unwrap(), vec![0x01, 0xFF]);
        assert_eq!(hex_to_bytes("").unwrap(), Vec::<u8>::new());
        assert!(hex_to_bytes("0").is_err()); // Odd length
        assert!(hex_to_bytes("GG").is_err()); // Invalid hex
    }

    #[test]
    fn test_extract_ruuvi_from_ble() {
        let ble_data = "99040512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
        let payload = extract_ruuvi_from_ble(ble_data).expect("Failed to extract Ruuvi data");
        assert_eq!(
            payload,
            "0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F".to_string()
        );
        assert_eq!(payload.len(), v5::PAYLOAD_WITH_MAC_LENGTH * 2);
        assert!(payload.starts_with("05")); // Data Format 5

        // Test with 0499 pattern (little-endian)
        let ble_data_le = "04990512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
        let extracted_le = extract_ruuvi_from_ble(ble_data_le);
        assert!(extracted_le.is_some());

        // Test with no Ruuvi data
        let non_ruuvi = "020106030316910255AA";
        assert!(extract_ruuvi_from_ble(non_ruuvi).is_none());
    }

    #[test]
    fn test_decode_empty_data() {
        assert!(decode("").is_err());
    }

    #[test]
    fn test_unsupported_format() {
        // Format 99 doesn't exist
        let result = decode("63000000000000000000000000000000000000000000000000");
        assert!(matches!(result, Err(DecodeError::UnsupportedFormat(99))));
    }

    #[test]
    fn test_decoding_ruuvi_data() {
        let ble_data = "99040512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
        let payload = decode(&extract_ruuvi_from_ble(ble_data).expect("ble_data"))
            .expect("Failed to extract Ruuvi data");
        match payload {
            RuuviData::V5(_) => (),
            _ => panic!("Unexpected data format"),
        }

        let ble_data = "990406170C5668C79E007000C90501D9FFCD004C884F";
        let payload = decode(&extract_ruuvi_from_ble(ble_data).expect("ble_data"))
            .expect("Failed to extract Ruuvi data");
        match payload {
            RuuviData::V6(_) => (),
            _ => panic!("Unexpected data format"),
        }

        let ble_data =
            "9904E1170C5668C79E0065007004BD11CA00C90A0213E0AC000000DECDEE100000000000CBB8334C884F";
        let payload = decode(&extract_ruuvi_from_ble(ble_data).expect("ble_data"))
            .expect("Failed to extract Ruuvi data");
        match payload {
            RuuviData::E1(_) => (),
            _ => panic!("Unexpected data format"),
        }
    }
}
