//! Data Format 5 (`RAWv2`) decoder implementation
//!
//! This module implements decoding for Ruuvi Data Format 5 based on the official specification:
//! <https://docs.ruuvi.com/communication/bluetooth-advertisements/data-format-5-rawv2>

use serde::{Deserialize, Serialize};

use crate::error::{DecodeError, Result};

/// Expected payload length for Data Format 5 in bytes
pub const PAYLOAD_LENGTH: usize = 18;
pub const PAYLOAD_WITH_MAC_LENGTH: usize = PAYLOAD_LENGTH + 6;

/// Data Format 5 (`RAWv2`) structure
///
/// This format contains all the sensor readings in a 24-byte payload
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataFormatV5 {
    /// MAC address as lowercase hex string (without colons)
    pub mac_address: String,
    /// Temperature in Celsius (-163.835 to +163.835°C, 0.005°C resolution)
    pub temperature: Option<f64>,
    /// Humidity in % (0 to 163.835%, 0.0025% resolution)
    pub humidity: Option<f64>,
    /// Pressure in hPa (500-1155.35 hPa, 0.01 hPa resolution)
    pub pressure: Option<f64>,
    /// Acceleration X-axis in millig (-32767 to +32767 mg, 1 mg resolution)
    pub acceleration_x: Option<i16>,
    /// Acceleration Y-axis in millig (-32767 to +32767 mg, 1 mg resolution)
    pub acceleration_y: Option<i16>,
    /// Acceleration Z-axis in millig (-32767 to +32767 mg, 1 mg resolution)
    pub acceleration_z: Option<i16>,
    /// Battery voltage in mV (1600-3646 mV, 1 mV resolution)
    pub battery_voltage: Option<u16>,
    /// TX power in dBm (-40 to +20 dBm, 2 dBm resolution)
    pub tx_power: Option<i8>,
    /// Movement counter (0-254, increments when motion detected)
    pub movement_counter: Option<u8>,
    /// Measurement sequence number (0-65534, increments with each measurement)
    pub measurement_sequence: Option<u16>,
}

/// Decode Data Format 5 payload from raw bytes
///
/// # Arguments
///
/// * `bytes` - Raw bytes starting with format identifier (should be 24 bytes total)
///
/// # Returns
///
/// * `Ok(DataFormatV5)` - Successfully decoded data
/// * `Err(DecodeError)` - Decoding failed
///
/// # Example
///
/// ```rust
/// use ruuvi_decoders::v5::decode;
///
/// let bytes = hex::decode("0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F").unwrap();
/// let result = decode(&bytes).unwrap();
/// assert_eq!(result.temperature, Some(24.3));
/// ```
///
/// # Errors
///
/// * `Err(DecodeError::InvalidLength)` - Input length is not 24 bytes
/// * `Err(DecodeError::UnsupportedFormat)` - Format identifier is not 5
/// * `Err(DecodeError::InvalidTemperature)` - Temperature value is invalid
/// * `Err(DecodeError::InvalidHumidity)` - Humidity value is invalid
/// * `Err(DecodeError::InvalidPressure)` - Pressure value is invalid
/// * `Err(DecodeError::InvalidAcceleration)` - Acceleration value is invalid
/// * `Err(DecodeError::InvalidPowerInfo)` - Power information is invalid
/// * `Err(DecodeError::InvalidMovementCounter)` - Movement counter is invalid
/// * `Err(DecodeError::InvalidMeasurementSequence)` - Measurement sequence is invalid
/// * `Err(DecodeError::InvalidMacAddress)` - MAC address is invalid
pub fn decode(bytes: &[u8]) -> Result<DataFormatV5> {
    // Validate input length
    if bytes.len() != PAYLOAD_WITH_MAC_LENGTH {
        return Err(DecodeError::invalid_length(
            PAYLOAD_WITH_MAC_LENGTH,
            bytes.len(),
        ));
    }

    // Validate format identifier
    if bytes[0] != 5 {
        return Err(DecodeError::UnsupportedFormat(bytes[0]));
    }

    // Extract all fields
    let temperature = decode_temperature(&bytes[1..3])?;
    let humidity = decode_humidity(&bytes[3..5])?;
    let pressure = decode_pressure(&bytes[5..7])?;
    let acceleration_x = decode_acceleration(&bytes[7..9])?;
    let acceleration_y = decode_acceleration(&bytes[9..11])?;
    let acceleration_z = decode_acceleration(&bytes[11..13])?;
    let (battery_voltage, tx_power) = decode_power_info(&bytes[13..15])?;
    let movement_counter = decode_movement_counter(bytes[15]);
    let measurement_sequence = decode_measurement_sequence(&bytes[16..18])?;
    let mac_address = decode_mac_address(&bytes[18..24]);

    Ok(DataFormatV5 {
        mac_address,
        temperature,
        humidity,
        pressure,
        acceleration_x,
        acceleration_y,
        acceleration_z,
        battery_voltage,
        tx_power,
        movement_counter,
        measurement_sequence,
    })
}

/// Decode temperature from 2 bytes
/// Range: -163.835°C to +163.835°C in 0.005°C increments
/// Invalid value: 0x8000 (-32768)
fn decode_temperature(bytes: &[u8]) -> Result<Option<f64>> {
    if bytes.len() != 2 {
        return Err(DecodeError::InvalidLength(
            "Temperature field must be 2 bytes".into(),
        ));
    }

    let raw_value = i16::from_be_bytes([bytes[0], bytes[1]]);

    if raw_value == i16::MIN {
        // 0x8000 = invalid/not available
        Ok(None)
    } else {
        // Resolution is 0.005°C
        let temperature = f64::from(raw_value) * 0.005;
        Ok(Some(temperature))
    }
}

/// Decode humidity from 2 bytes
/// Range: 0% to 163.835% in 0.0025% increments
/// Invalid value: 65535
fn decode_humidity(bytes: &[u8]) -> Result<Option<f64>> {
    if bytes.len() != 2 {
        return Err(DecodeError::InvalidLength(
            "Humidity field must be 2 bytes".into(),
        ));
    }

    let raw_value = u16::from_be_bytes([bytes[0], bytes[1]]);

    if raw_value == 65535 {
        // 0xFFFF = invalid/not available
        Ok(None)
    } else {
        // Resolution is 0.0025%
        let humidity = f64::from(raw_value) * 0.0025;
        Ok(Some(humidity))
    }
}

/// Decode pressure from 2 bytes
/// Range: 50000Pa to 115534Pa in 1Pa increments (with -50000Pa offset)
/// Invalid value: 65535
fn decode_pressure(bytes: &[u8]) -> Result<Option<f64>> {
    if bytes.len() != 2 {
        return Err(DecodeError::InvalidLength(
            "Pressure field must be 2 bytes".into(),
        ));
    }

    let raw_value = u16::from_be_bytes([bytes[0], bytes[1]]);

    if raw_value == 65535 {
        // 0xFFFF = invalid/not available
        Ok(None)
    } else {
        // Add offset of 50000Pa
        let pressure = f64::from(raw_value) + 50000.0;
        Ok(Some(pressure))
    }
}

/// Decode acceleration from 2 bytes
/// Range: -32767 to +32767 mG
/// Invalid value: -32768 (0x8000)
fn decode_acceleration(bytes: &[u8]) -> Result<Option<i16>> {
    if bytes.len() != 2 {
        return Err(DecodeError::InvalidLength(
            "Acceleration field must be 2 bytes".into(),
        ));
    }

    let raw_value = i16::from_be_bytes([bytes[0], bytes[1]]);

    if raw_value == i16::MIN {
        // 0x8000 = invalid/not available
        Ok(None)
    } else {
        Ok(Some(raw_value))
    }
}

/// Decode power info (battery voltage and TX power) from 2 bytes
/// Battery voltage: 11 bits (1600mV to 3647mV)
/// TX power: 5 bits (-40dBm to +20dBm in 2dBm steps)
/// Invalid values: 2047 for battery, 31 for TX power
fn decode_power_info(bytes: &[u8]) -> Result<(Option<u16>, Option<i8>)> {
    if bytes.len() != 2 {
        return Err(DecodeError::InvalidLength(
            "Power info field must be 2 bytes".into(),
        ));
    }

    let raw_value = u16::from_be_bytes([bytes[0], bytes[1]]);

    // Battery voltage: upper 11 bits
    let battery_raw = (raw_value >> 5) & 0x07FF; // Extract bits 15-5
    let battery_voltage = if battery_raw == 2047 {
        None // Invalid/not available
    } else {
        Some(battery_raw + 1600) // Add 1600mV offset
    };

    // TX power: lower 5 bits
    let tx_power_raw = (raw_value & 0x001F) as u8; // Extract bits 4-0
    let tx_power = if tx_power_raw == 31 {
        None // Invalid/not available
    } else {
        Some(-40 + (tx_power_raw.cast_signed()) * 2) // -40dBm + (value * 2dBm)
    };

    Ok((battery_voltage, tx_power))
}

/// Decode movement counter from 1 byte
/// Range: 0 to 254
/// Invalid value: 255
fn decode_movement_counter(byte: u8) -> Option<u8> {
    if byte == 255 {
        None // Invalid/not available
    } else {
        Some(byte)
    }
}

/// Decode measurement sequence number from 2 bytes
/// Range: 0 to 65534
/// Invalid value: 65535
fn decode_measurement_sequence(bytes: &[u8]) -> Result<Option<u16>> {
    if bytes.len() != 2 {
        return Err(DecodeError::InvalidLength(
            "Measurement sequence field must be 2 bytes".into(),
        ));
    }

    let raw_value = u16::from_be_bytes([bytes[0], bytes[1]]);

    if raw_value == 65535 {
        Ok(None) // Invalid/not available
    } else {
        Ok(Some(raw_value))
    }
}

/// Decode MAC address from 6 bytes to lowercase hex string
fn decode_mac_address(bytes: &[u8]) -> String {
    use std::fmt::Write;

    if bytes.len() != 6 {
        return "invalid".to_string();
    }

    // Check for invalid MAC (all 0xFF)
    if bytes.iter().all(|&b| b == 0xFF) {
        return "invalid".to_string();
    }

    bytes.iter().fold(String::new(), |mut output, b| {
        let _ = write!(output, "{b:02x}");
        output
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use rstest::rstest;

    // Keep struct-level decoding snapshot tests (insta) for readability of full structs.
    #[rstest]
    #[case::valid("valid", "0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F")]
    #[case::maximum("maximum", "057FFFFFFEFFFE7FFF7FFF7FFFFFDEFEFFFECBB8334C884F")]
    #[case::minimum("minimum", "058001000000008001800180010000000000CBB8334C884F")]
    #[case::invalid("invalid", "058000FFFFFFFF800080008000FFFFFFFFFFFFFFFFFFFFFF")]
    #[case::sea_level("sea_level", "0500004E20C8550000000000000000000001CBB8334C884F")]
    fn decode_snapshot(#[case] name: &str, #[case] hex_str: &str) {
        let raw = hex::decode(hex_str).unwrap();
        let res = decode(&raw).unwrap();
        // Snapshot the whole decoded `DataFormatV5` for these canonical payloads.
        assert_debug_snapshot!(name, res);
    }

    // Error and boundary checks remain explicit.
    #[test]
    fn decode_errors() {
        let short_data = vec![0x05, 0x12, 0xFC]; // Too short
        assert!(matches!(
            decode(&short_data),
            Err(DecodeError::InvalidLength(_))
        ));

        let long_data = vec![0u8; 30]; // Too long
        assert!(matches!(
            decode(&long_data),
            Err(DecodeError::InvalidLength(_))
        ));

        let wrong_format = vec![0x06; 24]; // Format 6, not 5
        assert!(matches!(
            decode(&wrong_format),
            Err(DecodeError::UnsupportedFormat(6))
        ));
    }

    // For primitive-returning decoders, use rstest with expected primitive values.
    #[rstest]
    #[case("0000", Some(0.0))]
    #[case("01C3", Some(2.255))]
    #[case("FE3D", Some(-2.255))]
    #[case("8000", None)]
    fn temperature_cases(#[case] hex_str: &str, #[case] expected: Option<f64>) {
        let bytes = hex::decode(hex_str).unwrap();
        assert_eq!(decode_temperature(&bytes).unwrap(), expected);
    }

    #[rstest]
    #[case("0000", Some(0.0))]
    #[case("2710", Some(25.0))]
    #[case("9C40", Some(100.0))]
    #[case("FFFF", None)]
    fn humidity_cases(#[case] hex_str: &str, #[case] expected: Option<f64>) {
        let bytes = hex::decode(hex_str).unwrap();
        assert_eq!(decode_humidity(&bytes).unwrap(), expected);
    }

    #[rstest]
    #[case("0000", Some(50000.0))]
    #[case("C855", Some(101285.0))]
    #[case("FFFE", Some(115534.0))]
    #[case("FFFF", None)]
    fn pressure_cases(#[case] hex_str: &str, #[case] expected: Option<f64>) {
        let bytes = hex::decode(hex_str).unwrap();
        assert_eq!(decode_pressure(&bytes).unwrap(), expected);
    }

    #[rstest]
    #[case("03E8", Some(1000))]
    #[case("FC18", Some(-1000))]
    #[case("8000", None)]
    fn acceleration_cases(#[case] hex_str: &str, #[case] expected: Option<i16>) {
        let bytes = hex::decode(hex_str).unwrap();
        assert_eq!(decode_acceleration(&bytes).unwrap(), expected);
    }

    #[rstest]
    #[case("0AC3", (Some(1686u16), Some(-34i8)))]
    #[case("FFE0", (None, Some(-40i8)))] // battery invalid; lower bits E0 -> 0 => -40 dBm
    #[case("FFFF", (None, None))]
    fn power_info_cases(#[case] hex_str: &str, #[case] expected: (Option<u16>, Option<i8>)) {
        let bytes = hex::decode(hex_str).unwrap();
        assert_eq!(decode_power_info(&bytes).unwrap(), expected);
    }

    #[rstest]
    #[case([0xCB, 0xB8, 0x33, 0x4C, 0x88, 0x4F], "cbb8334c884f")]
    #[case([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], "invalid")]
    fn mac_address_cases(#[case] input: [u8; 6], #[case] expected: &str) {
        assert_eq!(decode_mac_address(&input), expected.to_string());
    }

    #[test]
    fn movement_and_sequence_boundaries() {
        // Movement counter boundary
        assert_eq!(decode_movement_counter(254), Some(254));
        assert_eq!(decode_movement_counter(255), None);

        // Measurement sequence boundary
        assert_eq!(
            decode_measurement_sequence(&[0xFF, 0xFE]).unwrap(),
            Some(65534)
        );
        assert_eq!(decode_measurement_sequence(&[0xFF, 0xFF]).unwrap(), None);
    }
}
