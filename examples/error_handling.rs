//! Error Handling Example for Ruuvi Decoders
//!
//! This example demonstrates comprehensive error handling patterns when working
//! with the ruuvi-decoders library, including recovery strategies and logging.
//!
//! Run with: cargo run --example error_handling

use ruuvi_decoders::{DecodeError, RuuviData, decode, extract_ruuvi_from_ble};
use std::collections::HashMap;

/// Statistics for error tracking
#[derive(Debug, Default)]
struct ErrorStats {
    total_attempts: u32,
    successful_decodes: u32,
    invalid_hex_errors: u32,
    invalid_length_errors: u32,
    unsupported_format_errors: u32,
    validation_errors: u32,
    other_errors: u32,
}

impl ErrorStats {
    fn record_success(&mut self) {
        self.total_attempts += 1;
        self.successful_decodes += 1;
    }

    fn record_error(&mut self, error: &DecodeError) {
        self.total_attempts += 1;
        match error {
            DecodeError::InvalidHex(_) => self.invalid_hex_errors += 1,
            DecodeError::InvalidLength(_) => self.invalid_length_errors += 1,
            DecodeError::UnsupportedFormat(_) => self.unsupported_format_errors += 1,
            DecodeError::ValidationFailed(_) => self.validation_errors += 1,
            _ => self.other_errors += 1,
        }
    }

    fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.0
        } else {
            (self.successful_decodes as f64 / self.total_attempts as f64) * 100.0
        }
    }
}

/// Robust decoder that handles errors gracefully
struct RobustDecoder {
    stats: ErrorStats,
    error_log: Vec<(String, DecodeError)>,
}

impl RobustDecoder {
    fn new() -> Self {
        Self {
            stats: ErrorStats::default(),
            error_log: Vec::new(),
        }
    }

    /// Attempt to decode Ruuvi data with comprehensive error handling
    fn decode_with_recovery(&mut self, input: &str) -> Option<RuuviData> {
        // Try direct decoding first
        match decode(input) {
            Ok(data) => {
                self.stats.record_success();
                Some(data)
            }
            Err(e) => {
                self.stats.record_error(&e);
                self.error_log.push((input.to_string(), e.clone()));

                // Attempt recovery strategies
                self.attempt_recovery(input, e)
            }
        }
    }

    /// Attempt various recovery strategies for failed decodes
    fn attempt_recovery(&mut self, input: &str, original_error: DecodeError) -> Option<RuuviData> {
        match original_error {
            DecodeError::InvalidHex(_) => {
                println!("🔧 Attempting hex cleanup for: {}", input);
                self.try_hex_cleanup(input)
            }
            DecodeError::InvalidLength(_) => {
                println!("🔧 Attempting length correction for: {}", input);
                self.try_length_correction(input)
            }
            DecodeError::UnsupportedFormat(format) => {
                println!(
                    "🔧 Unsupported format 0x{:02X}, checking if it's a future format",
                    format
                );
                None // No recovery possible for unsupported formats
            }
            _ => {
                println!("🔧 No recovery strategy available for: {}", original_error);
                None
            }
        }
    }

    /// Try to clean up hex string format issues
    fn try_hex_cleanup(&mut self, input: &str) -> Option<RuuviData> {
        let cleanup_attempts = vec![
            input.trim().to_string(), // Remove whitespace
            input.replace(" ", ""),   // Remove spaces
            input.replace(":", ""),   // Remove colons
            input.replace("-", ""),   // Remove dashes
            input.to_uppercase(),     // Try uppercase
            input.to_lowercase(),     // Try lowercase
        ];

        for cleaned in cleanup_attempts {
            if let Ok(data) = decode(&cleaned) {
                println!("✅ Recovery successful with cleanup: {}", cleaned);
                self.stats.record_success();
                return Some(data);
            }
        }

        None
    }

    /// Try to correct length issues
    fn try_length_correction(&mut self, input: &str) -> Option<RuuviData> {
        let input_len = input.len();

        // If too short, maybe it's missing leading zeros
        if input_len < 48 {
            let padded = format!("{:0>48}", input);
            if let Ok(data) = decode(&padded) {
                println!("✅ Recovery successful with zero-padding: {}", padded);
                self.stats.record_success();
                return Some(data);
            }
        }

        // If too long, maybe there's extra data at the end
        if input_len > 48 {
            let truncated = &input[..48];
            if let Ok(data) = decode(truncated) {
                println!("✅ Recovery successful with truncation: {}", truncated);
                self.stats.record_success();
                return Some(data);
            }

            // Maybe the Ruuvi data is embedded somewhere in the string
            for start in 0..(input_len - 47) {
                if start + 48 <= input_len {
                    let candidate = &input[start..start + 48];
                    if let Ok(data) = decode(candidate) {
                        println!(
                            "✅ Recovery successful by finding embedded data: {}",
                            candidate
                        );
                        self.stats.record_success();
                        return Some(data);
                    }
                }
            }
        }

        None
    }

    fn print_statistics(&self) {
        println!("\n📊 Decoder Statistics");
        println!("=====================");
        println!("Total decode attempts: {}", self.stats.total_attempts);
        println!("Successful decodes: {}", self.stats.successful_decodes);
        println!("Success rate: {:.1}%", self.stats.success_rate());
        println!("\nError Breakdown:");
        println!("  Invalid hex: {}", self.stats.invalid_hex_errors);
        println!("  Invalid length: {}", self.stats.invalid_length_errors);
        println!(
            "  Unsupported format: {}",
            self.stats.unsupported_format_errors
        );
        println!("  Validation failed: {}", self.stats.validation_errors);
        println!("  Other errors: {}", self.stats.other_errors);
    }

    fn print_error_log(&self) {
        if self.error_log.is_empty() {
            return;
        }

        println!("\n📝 Error Log (Recent Failures)");
        println!("===============================");

        // Group errors by type for better analysis
        let mut error_groups: HashMap<String, Vec<&str>> = HashMap::new();

        for (input, error) in &self.error_log {
            let error_type = match error {
                DecodeError::InvalidHex(_) => "Invalid Hex",
                DecodeError::InvalidLength(_) => "Invalid Length",
                DecodeError::UnsupportedFormat(_) => "Unsupported Format",
                DecodeError::ValidationFailed(_) => "Validation Failed",
                _ => "Other",
            };

            error_groups
                .entry(error_type.to_string())
                .or_insert_with(Vec::new)
                .push(input);
        }

        for (error_type, inputs) in error_groups {
            println!("\n{} ({} cases):", error_type, inputs.len());
            for (i, input) in inputs.iter().take(3).enumerate() {
                // Show first 3 examples
                println!("  {}: {}", i + 1, input);
            }
            if inputs.len() > 3 {
                println!("  ... and {} more", inputs.len() - 3);
            }
        }
    }
}

fn main() {
    println!("⚠️  Ruuvi Decoders - Error Handling Example");
    println!("==========================================\n");

    let mut decoder = RobustDecoder::new();

    // Test case 1: Valid data (should succeed)
    println!("🧪 Test 1: Valid Data");
    println!("---------------------");
    let valid_data = "0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
    if let Some(data) = decoder.decode_with_recovery(valid_data) {
        match data {
            RuuviData::V5(data_format_v5) => println!("✅ v5: {data_format_v5:?}"),
            RuuviData::V6(data_format_v6) => println!("✅ v6: {data_format_v6:?}"),
            RuuviData::E1(data_format_e1) => println!("✅ e1: {data_format_e1:?}"),
        }
    }

    // Test case 2: Invalid hex characters
    println!("\n🧪 Test 2: Invalid Hex Characters");
    println!("---------------------------------");
    let invalid_hex = "05G2FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
    decoder.decode_with_recovery(invalid_hex);

    // Test case 3: Wrong length (too short)
    println!("\n🧪 Test 3: Too Short");
    println!("-------------------");
    let too_short = "0512FC5394C37C";
    decoder.decode_with_recovery(too_short);

    // Test case 4: Wrong length (too long)
    println!("\n🧪 Test 4: Too Long");
    println!("------------------");
    let too_long = "0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884FEXTRABYTES";
    decoder.decode_with_recovery(too_long);

    // Test case 5: Unsupported format
    println!("\n🧪 Test 5: Unsupported Format");
    println!("----------------------------");
    let unsupported = "FF12FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
    decoder.decode_with_recovery(unsupported);

    // Test case 6: Hex with formatting (spaces, colons)
    println!("\n🧪 Test 6: Formatted Hex (Recovery Test)");
    println!("---------------------------------------");
    let formatted_hex = "05 12 FC 53 94 C3 7C 00 04 FF FC 04 0C AC 36 42 00 CD CB B8 33 4C 88 4F";
    decoder.decode_with_recovery(formatted_hex);

    // Test case 7: BLE advertisement parsing errors
    println!("\n🧪 Test 7: BLE Advertisement Parsing");
    println!("-----------------------------------");
    demonstrate_ble_parsing_errors(&mut decoder);

    // Test case 8: Batch processing with mixed data quality
    println!("\n🧪 Test 8: Batch Processing");
    println!("--------------------------");
    batch_processing_test(&mut decoder);

    // Print final statistics
    decoder.print_statistics();
    decoder.print_error_log();

    println!("\n✅ Error handling example completed!");
    println!("This example shows how to build resilient systems that can handle");
    println!("various types of input errors and attempt recovery when possible.");
}

/// Demonstrate BLE advertisement parsing error handling
fn demonstrate_ble_parsing_errors(decoder: &mut RobustDecoder) {
    let ble_test_cases = vec![
        (
            "Valid BLE",
            "02010603031691FF990405012FC5394C37C0004FFFC040CAC364200CDCBB8334C884F",
        ),
        ("No Ruuvi data", "020106030316910255AA"),
        ("Invalid BLE hex", "02010G03031691FF99"),
        ("Truncated BLE", "02010603031691FF99"),
    ];

    for (description, ble_data) in ble_test_cases {
        println!("  Testing: {}", description);

        match extract_ruuvi_from_ble(ble_data) {
            Some(ruuvi_hex) => {
                println!("    ✅ Extracted Ruuvi data: {}", ruuvi_hex);
                decoder.decode_with_recovery(&ruuvi_hex);
            }
            None => {
                println!("    ❌ No Ruuvi data found in BLE advertisement");
            }
        }
    }
}

/// Test batch processing with mixed data quality
fn batch_processing_test(decoder: &mut RobustDecoder) {
    let batch_data = vec![
        "0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F", // Valid
        "0512FC5394C37C0004FFFC040CAC364200CDCBB8334C88",   // Too short
        "G512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F", // Invalid hex
        "057FFFFFFEFFFE7FFF7FFF7FFFFFDEFEFFFECBB8334C884F", // Valid (max values)
        "FF12FC5394C37C0004FFFC040CAC364200CDCBB8334C884F", // Unsupported format
        "05 12 FC 53 94 C3 7C 00 04 FF FC 04 0C AC 36 42 00 CD CB B8 33 4C 88 4F", // Valid with spaces
    ];

    println!("  Processing {} data samples...", batch_data.len());

    let mut successful_decodes = 0;
    for (i, data) in batch_data.iter().enumerate() {
        print!("    Sample {}: ", i + 1);
        if let Some(decoded) = decoder.decode_with_recovery(data) {
            successful_decodes += 1;
            match decoded {
                RuuviData::V5(data_format_v5) => println!("✅ v5: {data_format_v5:?}"),
                RuuviData::V6(data_format_v6) => println!("✅ v6: {data_format_v6:?}"),
                RuuviData::E1(data_format_e1) => println!("✅ e1: {data_format_e1:?}"),
            }
        } else {
            println!("❌ Failed to decode");
        }
    }

    println!(
        "  Batch result: {}/{} successful",
        successful_decodes,
        batch_data.len()
    );
}
