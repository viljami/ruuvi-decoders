use crate::error::{DecodeError, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

pub const PAYLOAD_LENGTH: usize = 34;
pub const PAYLOAD_WITH_MAC_LENGTH: usize = PAYLOAD_LENGTH + 6;

/// Data Format E1 (Extended v1) structure, as specified in the Ruuvi E1 XML spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataFormatE1 {
    /// Temperature in Celsius (-163.835 to +163.835°C, 0.005°C resolution)
    pub temperature: Option<f64>,
    /// Humidity in % (0 to 100%, 0.0025% resolution, but up to 163.83% possible)
    pub humidity: Option<f64>,
    /// Pressure in hPa (500.00 to 1155.34 hPa, 1 Pa resolution, offset -50000 Pa)
    pub pressure: Option<f64>,
    /// PM1.0 in μg/m³ (0.1 μg/m³ resolution, 0..1000 μg/m³)
    pub pm1_0: Option<f64>,
    /// PM2.5 in μg/m³ (0.1 μg/m³ resolution, 0..1000 μg/m³)
    pub pm2_5: Option<f64>,
    /// PM4.0 in μg/m³ (0.1 μg/m³ resolution, 0..1000 μg/m³)
    pub pm4_0: Option<f64>,
    /// PM10.0 in μg/m³ (0.1 μg/m³ resolution, 0..1000 μg/m³)
    pub pm10_0: Option<f64>,
    /// CO2 concentration in ppm (1 ppm resolution, 0..40000 ppm)
    pub co2: Option<u16>,
    /// VOC index, unitless (1/bit, 0..500, 9 bits: 8 from byte + 1 from flags)
    pub voc_index: Option<u16>,
    /// `NOx` index, unitless (1/bit, 0..500, 9 bits: 8 from byte + 1 from flags)
    pub nox_index: Option<u16>,
    /// Luminosity, Lux (0.01 Lux/bit, 24 bits, 0..144284 Lux)
    pub luminosity: Option<f64>,
    /// Measurement sequence number (0..16777214, 24 bits)
    pub measurement_sequence: Option<u32>,
    /// Flags byte (bitfield, raw)
    pub flags: u8,
    /// MAC address as lowercase hex string (6 bytes, 48 bits)
    pub mac_address: String,
}

/// Decode Data Format E1 payload from raw bytes
///
/// # Arguments
///
/// * `bytes` - Raw bytes starting with format identifier (should be 40 bytes total)
///
/// # Returns
///
/// * `Ok(DataFormatE1)` - Successfully decoded data
/// * `Err(DecodeError)` - Decoding failed
///
/// # Errors
///
/// * `DecodeError::InvalidLength` - Invalid payload length
/// * `DecodeError::UnsupportedFormat` - Unsupported format identifier
#[allow(clippy::too_many_lines)]
#[allow(clippy::similar_names)]
pub fn decode(bytes: &[u8]) -> Result<DataFormatE1> {
    if bytes.len() != PAYLOAD_WITH_MAC_LENGTH {
        return Err(DecodeError::invalid_length(
            PAYLOAD_WITH_MAC_LENGTH,
            bytes.len(),
        ));
    }

    // Validate format identifier
    if bytes[0] != 0xE1 {
        return Err(DecodeError::UnsupportedFormat(bytes[0]));
    }

    // Helper closures for field extraction
    let get_i16 = |start| i16::from_be_bytes([bytes[start], bytes[start + 1]]);
    let get_u16 = |start| u16::from_be_bytes([bytes[start], bytes[start + 1]]);
    let get_u32 = |start| {
        (u32::from(bytes[start]) << 16)
            | (u32::from(bytes[start + 1]) << 8)
            | u32::from(bytes[start + 2])
    };

    // Temperature: 0.005°C/bit, i16, bytes 1-2
    let raw_temp = get_i16(1);
    let temperature = if raw_temp == i16::MIN {
        None
    } else {
        Some(f64::from(raw_temp) * 0.005)
    };

    // Humidity: 0.0025%/bit, u16, bytes 3-4
    let raw_humidity = get_u16(3);
    let humidity = if raw_humidity == 65535 {
        None
    } else {
        Some(f64::from(raw_humidity) * 0.0025)
    };

    // Pressure: 1 Pa/bit, offset -50000 Pa, u16, bytes 5-6
    let raw_pressure = get_u16(5);
    let pressure = if raw_pressure == 65535 {
        None
    } else {
        let pa = i32::from(raw_pressure) + 50000;
        Some(f64::from(pa) / 100.0) // Convert Pa to hPa
    };

    // PM1.0: 0.1 μg/m³/bit, u16, bytes 7-8
    let raw_pm1_0 = get_u16(7);
    let pm1_0 = if raw_pm1_0 == 0xFFFF {
        None
    } else {
        Some(f64::from(raw_pm1_0) * 0.1)
    };

    // PM2.5: 0.1 μg/m³/bit, u16, bytes 9-10
    let raw_pm2_5 = get_u16(9);
    let pm2_5 = if raw_pm2_5 == 0xFFFF {
        None
    } else {
        Some(f64::from(raw_pm2_5) * 0.1)
    };

    // PM4.0: 0.1 μg/m³/bit, u16, bytes 11-12
    let raw_pm4_0 = get_u16(11);
    let pm4_0 = if raw_pm4_0 == 0xFFFF {
        None
    } else {
        Some(f64::from(raw_pm4_0) * 0.1)
    };

    // PM10.0: 0.1 μg/m³/bit, u16, bytes 13-14
    let raw_pm10_0 = get_u16(13);
    let pm10_0 = if raw_pm10_0 == 0xFFFF {
        None
    } else {
        Some(f64::from(raw_pm10_0) * 0.1)
    };

    // CO2: 1 ppm/bit, u16, bytes 15-16
    let raw_co2 = get_u16(15);
    let co2 = if raw_co2 == 0xFFFF {
        None
    } else {
        Some(raw_co2)
    };

    // VOC index: 9 bits, byte 17 (hi) + flags b6 (LSB, bit 6 of byte 28)
    let raw_voc_hi = u16::from(bytes[17]);
    let voc_flag = (u16::from(bytes[28]) & 0b0100_0000) >> 6;
    let voc_index = {
        let value = (raw_voc_hi << 1) | voc_flag;
        if value > 500 { None } else { Some(value) }
    };

    // NOx index: 9 bits, byte 18 (hi) + flags b7 (LSB, bit 7 of byte 28)
    let raw_nox_hi = u16::from(bytes[18]);
    let nox_flag = (u16::from(bytes[28]) & 0b1000_0000) >> 7;
    let nox_index = {
        let value = (raw_nox_hi << 1) | nox_flag;
        if value > 500 { None } else { Some(value) }
    };

    // Luminosity: 0.01 Lux/bit, u24, bytes 19-21
    let raw_lum = get_u32(19);
    let luminosity = if raw_lum == 0x00FF_FFFF {
        None
    } else {
        Some(f64::from(raw_lum) * 0.01)
    };

    // Measurement sequence: u24, bytes 25-27
    let raw_seq = get_u32(25);
    let measurement_sequence = if raw_seq == 0x00FF_FFFF {
        None
    } else {
        Some(raw_seq)
    };

    // Flags: byte 28
    let flags = bytes[28];

    // MAC address: last 6 bytes (41..47)
    let mac_bytes = &bytes[PAYLOAD_LENGTH..PAYLOAD_WITH_MAC_LENGTH];
    let mac_address = mac_bytes.iter().fold(String::new(), |mut output, b| {
        let _ = write!(output, "{b:02x}");
        output
    });

    Ok(DataFormatE1 {
        temperature,
        humidity,
        pressure,
        pm1_0,
        pm2_5,
        pm4_0,
        pm10_0,
        co2,
        voc_index,
        nox_index,
        luminosity,
        measurement_sequence,
        flags,
        mac_address,
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

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
    fn test_decode_wrong_format() {
        let mut bytes: [u8; PAYLOAD_WITH_MAC_LENGTH] = [0; PAYLOAD_WITH_MAC_LENGTH];
        bytes[0] = 0x06;
        let err = decode(&bytes).unwrap_err();
        match err {
            DecodeError::UnsupportedFormat(0x06) => {}
            _ => panic!("Expected UnsupportedFormat error"),
        }
    }

    #[rstest]
    #[case::valid(
        "valid",
        "E1170C5668C79E0065007004BD11CA00C90A0213E0AC000000DECDEE100000000000CBB8334C884F"
    )]
    #[case::maximum(
        "maximum",
        "E1800100000000000000000000000000000000000000000000000000000000000000CBB8334C884F"
    )]
    #[case::minimum(
        "minimum",
        "E17FFF9C40FFFE27102710271027109C40FAFADC28F0000000FFFFFE3F0000000000CBB8334C884F"
    )]
    fn decode_snapshot(#[case] name: &str, #[case] hex_str: &str) {
        use insta::assert_debug_snapshot;

        let raw = hex::decode(hex_str).unwrap();
        let res = decode(&raw).unwrap();
        // Snapshot the whole decoded `DataFormatV5` for these canonical payloads.
        assert_debug_snapshot!(name, res);
    }
}
