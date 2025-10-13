//! Error types for Ruuvi decoders

use thiserror::Error;

/// Result type alias for decoder operations
pub type Result<T> = std::result::Result<T, DecodeError>;

/// Errors that can occur during Ruuvi data decoding
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DecodeError {
    /// Invalid hex string format
    #[error("Invalid hex string: {0}")]
    InvalidHex(String),

    /// Data length is invalid for the format
    #[error("Invalid data length: {0}")]
    InvalidLength(String),

    /// Unsupported data format
    #[error("Unsupported data format: 0x{0:02X}")]
    UnsupportedFormat(u8),

    /// Invalid data values (e.g., reserved values that indicate invalid readings)
    #[error("Invalid data values: {0}")]
    InvalidData(String),

    /// Checksum or validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Decryption failed (for encrypted formats like E1)
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// Missing required fields
    #[error("Missing required field: {0}")]
    MissingField(String),
}

impl DecodeError {
    /// Create a new `InvalidLength` error
    #[must_use]
    pub fn invalid_length(expected: usize, actual: usize) -> Self {
        Self::InvalidLength(format!("Expected {expected} bytes, got {actual}"))
    }

    /// Create a new `InvalidData` error for a specific field
    #[must_use]
    pub fn invalid_field(field: &str, value: &str) -> Self {
        Self::InvalidData(format!("Invalid {field} value: {value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DecodeError::UnsupportedFormat(99);
        assert_eq!(err.to_string(), "Unsupported data format: 0x63");

        let err = DecodeError::invalid_length(24, 20);
        assert_eq!(
            err.to_string(),
            "Invalid data length: Expected 24 bytes, got 20"
        );

        let err = DecodeError::invalid_field("temperature", "-163.84");
        assert_eq!(
            err.to_string(),
            "Invalid data values: Invalid temperature value: -163.84"
        );
    }

    #[test]
    fn test_error_equality() {
        let err1 = DecodeError::UnsupportedFormat(5);
        let err2 = DecodeError::UnsupportedFormat(5);
        let err3 = DecodeError::UnsupportedFormat(6);

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }
}
