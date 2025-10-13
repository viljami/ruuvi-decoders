use crate::error::{DecodeError, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// Expected payload length for Data Format 6 in bytes
pub const PAYLOAD_LENGTH: usize = 17;
pub const PAYLOAD_WITH_MAC_LENGTH: usize = PAYLOAD_LENGTH + 3; // 3 for compactness

/// Data Format 6 (`RAWv3`) structure, as specified in the Ruuvi v6 XML spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataFormatV6 {
    /// Temperature in Celsius (-163.835 to +163.835°C, 0.005°C resolution)
    pub temperature: Option<f64>,
    /// Humidity in % (0 to 100%, 0.0025% resolution, but up to 163.83% possible)
    pub humidity: Option<f64>,
    /// Pressure in hPa (500.00 to 1155.35 hPa, 1 Pa resolution, offset -50000 Pa)
    pub pressure: Option<f64>,
    /// PM2.5 in μg/m³ (0.1 μg/m³ resolution, 0..1000 μg/m³)
    pub pm2_5: Option<f64>,
    /// CO2 concentration in ppm (1 ppm resolution, 0..40000 ppm)
    pub co2: Option<u16>,
    /// VOC index, unitless (1/bit, 0..500, 9 bits: 8 from byte + 1 from flags)
    pub voc_index: Option<u16>,
    /// `NOx` index, unitless (1/bit, 0..500, 9 bits: 8 from byte + 1 from flags)
    pub nox_index: Option<u16>,
    /// Luminosity, Lux (logarithmic, see spec for decoding)
    pub luminosity: Option<f64>,
    /// Reserved (should be 255, but included for completeness)
    pub reserved: Option<u8>,
    /// Measurement sequence number (0..255)
    pub measurement_sequence: Option<u8>,
    /// Flags byte (bitfield, raw)
    pub flags: u8,
    /// MAC address as lowercase hex string (3 bytes, 24 bits)
    pub mac_address: String,
}

/// Decode Data Format 6 payload from raw bytes
///
/// # Arguments
///
/// * `bytes` - Raw bytes starting with format identifier (should be 17 bytes total)
///
/// # Returns
///
/// * `Ok(DataFormatV6)` - Successfully decoded data
/// * `Err(DecodeError)` - Decoding failed
///
/// # Errors
///
/// * `DecodeError::InvalidLength` - Payload length is not 17 bytes
/// * `DecodeError::UnsupportedFormat` - Format identifier is not 6
pub fn decode(bytes: &[u8]) -> Result<DataFormatV6> {
    if bytes.len() != PAYLOAD_WITH_MAC_LENGTH {
        return Err(DecodeError::invalid_length(
            PAYLOAD_WITH_MAC_LENGTH,
            bytes.len(),
        ));
    }

    // Validate format identifier
    if bytes[0] != 6 {
        return Err(DecodeError::UnsupportedFormat(bytes[0]));
    }

    // Helper closures for field extraction
    let get_i16 = |start| i16::from_be_bytes([bytes[start], bytes[start + 1]]);
    let get_u16 = |start| u16::from_be_bytes([bytes[start], bytes[start + 1]]);

    // Temperature: 0.005°C/bit, i16, bytes 1-2
    let raw_temp = get_i16(1);
    let temperature = if raw_temp == i16::MIN {
        None
    } else {
        Some(f64::from(raw_temp) * 0.005)
    };

    // Humidity: 0.0025%/bit, u16, bytes 3-4
    let raw_humidity = get_u16(3);
    let humidity = if raw_humidity > 40000 {
        None
    } else {
        Some(f64::from(raw_humidity) * 0.0025)
    };

    // Pressure: 1 Pa/bit, offset +50000 Pa, u16, bytes 5-6
    let raw_pressure = get_u16(5);
    let pressure = if raw_pressure == 65535 {
        None
    } else {
        let pa = i32::from(raw_pressure) + 50000;
        Some(f64::from(pa) / 100.0) // Convert Pa to hPa
    };

    // PM2.5: 0.1 μg/m³/bit, u16, bytes 7-8
    let raw_pm2_5 = get_u16(7);
    let pm2_5 = if raw_pm2_5 > 10000 {
        None
    } else {
        Some(f64::from(raw_pm2_5) * 0.1)
    };

    // CO2: 1 ppm/bit, u16, bytes 9-10
    let raw_co2 = get_u16(9);
    let co2 = if raw_co2 > 40000 { None } else { Some(raw_co2) };

    // VOC index: 9 bits, bytes 11 (hi) + flags b6 (LSB)
    let raw_voc_hi = u16::from(bytes[11]);
    let voc_flag = (u16::from(bytes[16]) & 0b0100_0000) >> 6;
    let voc_index = {
        let value = (raw_voc_hi << 1) | voc_flag;
        if value > 500 {
            None
        } else {
            Some(value)
        }
    };

    // NOx index: 9 bits, bytes 12 (hi) + flags b7 (LSB)
    let raw_nox_hi = u16::from(bytes[12]);
    let nox_flag = (u16::from(bytes[16]) & 0b1000_0000) >> 7;
    let nox_index = {
        let value = (raw_nox_hi << 1) | nox_flag;
        if value > 500 {
            None
        } else {
            Some(value)
        }
    };

    // Luminosity: logarithmic, byte 13
    let raw_lum = bytes[13];
    let luminosity = if raw_lum == 255 {
        None
    } else {
        // MAX_VALUE := 65535
        // MAX_CODE  := 254
        // DELTA     := ln(MAX_VALUE + 1) / MAX_CODE

        // Encoding would be
        // CODE      := round(ln(value + 1) / DELTA)

        const MAX_VALUE: f64 = 65535.0;
        const MAX_CODE: f64 = 254.0;
        let delta: f64 = (MAX_VALUE + 1.0_f64).ln() / MAX_CODE;
        // Decoding:
        // VALUE     := exp(CODE * delta) - 1
        let value = (f64::from(raw_lum).round() * delta).exp() - 1.0;
        Some(value.min(MAX_VALUE))
    };

    println!("Luminosity: {raw_lum} => {:?}", luminosity);
    // Reserved: byte 14
    let reserved = Some(bytes[14]);

    // Measurement sequence: byte 15
    let measurement_sequence = Some(bytes[15]);

    // Flags: byte 16
    let flags = bytes[16];

    // MAC address: last 3 bytes 17 - 20
    let mac_bytes = &bytes[PAYLOAD_LENGTH..PAYLOAD_WITH_MAC_LENGTH];

    let mac_address = mac_bytes.iter().fold(String::new(), |mut output, b| {
        let _ = write!(output, "{b:02x}");
        output
    });

    Ok(DataFormatV6 {
        temperature,
        humidity,
        pressure,
        pm2_5,
        co2,
        voc_index,
        nox_index,
        luminosity,
        reserved,
        measurement_sequence,
        flags,
        mac_address,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Converts a hex string (with or without spaces, upper/lowercase) to a Vec<u8>.
    /// No padding or substitutions are performed. The string must contain the full payload including MAC.
    fn hex_str_to_bytes(s: &str) -> Vec<u8> {
        hex::decode(s).expect("Failed to decode hex string")
    }

    #[test]
    fn test_decode_valid_data() {
        let bytes = hex_str_to_bytes("06170C5668C79E007000C90501D9FFCD004C884F");
        assert_eq!(bytes.len(), PAYLOAD_WITH_MAC_LENGTH);
        let result = decode(&bytes).unwrap();
        assert_eq!(result.temperature, Some(29.5));
        assert!(result.humidity.unwrap() - 55.3 < 0.01);
        assert_eq!(result.pressure, Some(1011.02)); // -100.0 hPa is invalid
        assert!(result.pm2_5.unwrap() - 11.2 < 0.01);
        assert_eq!(result.co2, Some(201));
        assert_eq!(result.voc_index, Some(10));
        assert_eq!(result.nox_index, Some(2));
        assert!(result.luminosity.unwrap() - 13_026.67 < 0.01);
        assert_eq!(result.reserved, Some(0xFF));
        assert_eq!(result.measurement_sequence, Some(205));
        assert_eq!(result.flags, 0x00);
        assert_eq!(result.mac_address, "4c884f");
    }

    #[test]
    fn test_decode_invalid_length() {
        let bytes: [u8; 10] = [0; 10];
        let err = decode(&bytes).unwrap_err();
        match err {
            DecodeError::InvalidLength(_) => {}
            _ => panic!("Expected InvalidLength error"),
        }
    }

    #[test]
    fn test_decode_spec_valid_data() {
        let bytes = hex_str_to_bytes("06170C5668C79E007000C90501D9FFCD004C884F");
        assert_eq!(bytes.len(), PAYLOAD_WITH_MAC_LENGTH);
        let result = decode(&bytes).unwrap();
        assert!((result.temperature.unwrap() - 29.5).abs() < 0.01);
        assert!((result.pressure.unwrap() - 1011.02).abs() < 0.01);
        assert!((result.humidity.unwrap() - 55.3).abs() < 0.01);
        assert!((result.pm2_5.unwrap() - 11.2).abs() < 0.01);
        assert_eq!(result.co2, Some(201));
        assert_eq!(result.voc_index, Some(10));
        assert_eq!(result.nox_index, Some(2));
        // Luminosity: 0xD9, should decode to ~13026.67 Lux
        assert!((result.luminosity.unwrap() as f64 - 13026.67).abs() < 1.0);
        assert_eq!(result.measurement_sequence, Some(205));
        assert_eq!(result.reserved, Some(0xFF));
        assert_eq!(result.flags, 0x00);
        assert_eq!(result.mac_address, "4c884f");
    }

    #[test]
    fn test_decode_spec_maximum_values() {
        let bytes = hex_str_to_bytes("067FFF9C40FFFE27109C40FAFAFEFFFF074C8F4F");
        assert_eq!(bytes.len(), PAYLOAD_WITH_MAC_LENGTH);
        let result = decode(&bytes).unwrap();
        println!("{:?}", result);
        assert!((result.temperature.unwrap() - 163.835).abs() < 0.01);
        assert!((result.pressure.unwrap() - 1155.34).abs() < 0.01);
        assert!((result.humidity.unwrap() - 100.0).abs() < 0.01);
        assert!((result.pm2_5.unwrap() - 1000.0).abs() < 0.01);
        assert_eq!(result.co2, Some(40000));
        assert_eq!(result.voc_index, Some(500));
        assert_eq!(result.nox_index, Some(500));
        // Luminosity: 0xFE, should decode to ~65355.00 Lux
        assert!((result.luminosity.unwrap() - 65535.0).abs() < 1.0);
        assert_eq!(result.measurement_sequence, Some(255));
        assert_eq!(result.reserved, Some(0xFF));
        assert_eq!(result.flags, 0x07);
        assert_eq!(result.mac_address, "4c8f4f");
    }

    #[test]
    fn test_decode_spec_minimum_values() {
        let bytes = hex_str_to_bytes("06800100000000000000000000000000004C884F");
        assert_eq!(bytes.len(), PAYLOAD_WITH_MAC_LENGTH);
        let result = decode(&bytes).unwrap();
        assert_eq!(result.temperature, Some(-163.835));
        assert!((result.pressure.unwrap() - 500.0).abs() < 0.01);
        assert!((result.humidity.unwrap() - 0.0).abs() < 0.01);
        assert!((result.pm2_5.unwrap() - 0.0).abs() < 0.01);
        assert_eq!(result.co2, Some(0));
        assert_eq!(result.voc_index, Some(0));
        assert_eq!(result.nox_index, Some(0));
        assert_eq!(result.luminosity, Some(0.0_f64));
        assert_eq!(result.measurement_sequence, Some(0));
        assert_eq!(result.reserved, Some(0x00));
        assert_eq!(result.flags, 0x00);
        assert_eq!(result.mac_address, "4c884f");
    }

    #[test]
    fn test_decode_wrong_format() {
        let mut bytes: [u8; PAYLOAD_WITH_MAC_LENGTH] = [0; PAYLOAD_WITH_MAC_LENGTH];
        bytes[0] = 0x05;
        let err = decode(&bytes).unwrap_err();
        match err {
            DecodeError::UnsupportedFormat(0x05) => {}
            _ => panic!("Expected UnsupportedFormat error"),
        }
    }
}
