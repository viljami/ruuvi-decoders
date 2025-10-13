//! Data types for Ruuvi sensor data
//!
//! These types match the TypeScript interfaces in the shared package

use crate::error::{DecodeError, Result};
use crate::{
    e1::{self, DataFormatE1},
    v5::{self, DataFormatV5},
    v6::{self, DataFormatV6},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuuviGatewayEvent {
    pub gw_mac: String,
    pub rssi: i32,
    pub aoa: Vec<f64>,
    pub gwts: Option<u64>, // seconds since epoch
    pub ts: Option<u64>,   // seconds since epoch
    pub data: String,
    pub coords: Option<String>,
}

/// Supported Ruuvi data formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum DataFormat {
    /// Data Format 5 (`RAWv2`)
    V5 = 5,
    /// Data Format 6 (`RAWv3`) - TODO
    V6 = 6,
    /// Data Format E1 (Encrypted) - TODO
    E1 = 0xE1,
}

impl DataFormat {
    /// Create `DataFormat` from u8 value
    #[must_use]
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            5 => Some(Self::V5),
            6 => Some(Self::V6),
            0xE1 => Some(Self::E1),
            _ => None,
        }
    }

    /// Get the expected payload length in bytes for this format
    #[must_use]
    pub fn payload_length(&self) -> usize {
        match self {
            Self::V5 => v5::PAYLOAD_LENGTH,
            Self::V6 => v6::PAYLOAD_LENGTH,
            Self::E1 => e1::PAYLOAD_LENGTH,
        }
    }

    /// Get the expected payload length in bytes for this format
    #[must_use]
    pub fn payload_with_mac_length(&self) -> usize {
        match self {
            Self::V5 => v5::PAYLOAD_WITH_MAC_LENGTH,
            Self::V6 => v6::PAYLOAD_WITH_MAC_LENGTH,
            Self::E1 => e1::PAYLOAD_WITH_MAC_LENGTH,
        }
    }
}

/// Unified enum for all supported Ruuvi data formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "format")]
pub enum RuuviData {
    /// Data Format 5 (`RAWv2`)
    V5(DataFormatV5),
    /// Data Format 6 (`RAWv3`) - TODO
    #[allow(dead_code)]
    V6(DataFormatV6),
    /// Data Format E1 (Encrypted) - TODO
    #[allow(dead_code)]
    E1(DataFormatE1),
}

impl RuuviData {
    pub fn decode(data: &[u8]) -> Result<Self> {
        match data[0] {
            5 => Ok(Self::V5(v5::decode(data)?)),
            6 => Ok(Self::V6(v6::decode(data)?)),
            0xE1 => Ok(Self::E1(e1::decode(data)?)),
            other => Err(DecodeError::UnsupportedFormat(other)),
        }
    }
}
