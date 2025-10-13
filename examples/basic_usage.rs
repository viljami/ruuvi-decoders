//! Basic Usage Example for Ruuvi Decoders
//!
//! This example demonstrates the core functionality of the ruuvi-decoders library,
//! including BLE advertisement parsing, direct hex decoding, and error handling.
//!
//! Run with: cargo run --example basic_usage

use ruuvi_decoders::{RuuviData, decode, extract_ruuvi_from_ble};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 Ruuvi Decoders - Basic Usage Example\n");

    // Example 1: Full BLE Advertisement Parsing
    println!("📡 Example 1: Full BLE Advertisement");
    println!("==================================");

    let ble_advertisement = "02010603031691FF990405159F7C025A8BC4A53C00FB00000000E7FEE7FE00E7FE";
    println!("BLE Advertisement: {}", ble_advertisement);

    // Extract Ruuvi payload from BLE advertisement
    match extract_ruuvi_from_ble(ble_advertisement) {
        Some(ruuvi_hex) => {
            println!("Extracted Ruuvi payload: {}", ruuvi_hex);
            decode_and_display(&ruuvi_hex)?;
        }
        None => println!("❌ No Ruuvi data found in BLE advertisement"),
    }

    println!();

    // Example 2: Direct Hex Decoding (Official Test Vector)
    println!("🔧 Example 2: Direct Hex Decoding");
    println!("=================================");

    // This is the "valid data" test vector from Ruuvi documentation
    let test_vector = "0512FC5394C37C0004FFFC040CAC364200CDCBB8334C884F";
    println!("Test vector: {}", test_vector);
    decode_and_display(test_vector)?;

    println!();

    // Example 3: Maximum Values Test Vector
    println!("📊 Example 3: Maximum Values");
    println!("============================");

    let max_values = "057FFFFFFEFFFE7FFF7FFF7FFFFFDEFEFFFECBB8334C884F";
    println!("Max values vector: {}", max_values);
    decode_and_display(max_values)?;

    println!();

    // Example 4: Invalid Values Test Vector
    println!("❓ Example 4: Invalid/Unavailable Values");
    println!("=======================================");

    let invalid_values = "058000FFFFFFFF800080008000FFFFFFFFFFFFFFFFFFFFFF";
    println!("Invalid values vector: {}", invalid_values);
    decode_and_display(invalid_values)?;

    println!();

    // Example 5: Error Handling
    println!("⚠️  Example 5: Error Handling");
    println!("=============================");

    demonstrate_error_handling();

    Ok(())
}

/// Decode and display Ruuvi data in a formatted way
fn decode_and_display(hex_data: &str) -> Result<(), Box<dyn std::error::Error>> {
    match decode(hex_data) {
        Ok(ruuvi_data) => {
            print_ruuvi_data(&ruuvi_data);
            Ok(())
        }
        Err(e) => {
            println!("❌ Decoding failed: {}", e);
            Err(Box::new(e))
        }
    }
}

/// Pretty print Ruuvi data based on format
fn print_ruuvi_data(data: &RuuviData) {
    println!("✅ Successfully decoded:");
    match data {
        RuuviData::V5(data_format_v5) => println!("✅ v5: {data_format_v5:?}"),
        RuuviData::V6(data_format_v6) => println!("✅ v6: {data_format_v6:?}"),
        RuuviData::E1(data_format_e1) => println!("✅ e1: {data_format_e1:?}"),
    }

    match data {
        RuuviData::V5(v5_data) => {
            println!("   📊 Sensor Readings:");

            // Temperature (always present in valid data)
            match v5_data.temperature {
                Some(temp) => println!("     🌡️  Temperature: {:.3}°C", temp),
                None => println!("     🌡️  Temperature: Not available"),
            }

            // Humidity
            match v5_data.humidity {
                Some(humidity) => println!("     💧 Humidity: {:.2}%", humidity),
                None => println!("     💧 Humidity: Not available"),
            }

            // Pressure
            match v5_data.pressure {
                Some(pressure) => println!(
                    "     🌪️  Pressure: {:.0} Pa ({:.2} hPa)",
                    pressure,
                    pressure / 100.0
                ),
                None => println!("     🌪️  Pressure: Not available"),
            }

            // Acceleration
            println!("   🏃 Motion Data:");
            match (
                v5_data.acceleration_x,
                v5_data.acceleration_y,
                v5_data.acceleration_z,
            ) {
                (Some(x), Some(y), Some(z)) => {
                    println!("     📊 Acceleration: X={} mG, Y={} mG, Z={} mG", x, y, z);
                    let total =
                        ((x as f64).powi(2) + (y as f64).powi(2) + (z as f64).powi(2)).sqrt();
                    println!("     📏 Total acceleration: {:.0} mG", total);
                }
                _ => println!("     📊 Acceleration: Not available"),
            }

            match v5_data.movement_counter {
                Some(count) => println!("     🚶 Movement counter: {}", count),
                None => println!("     🚶 Movement counter: Not available"),
            }

            // Power info
            println!("   🔋 Power Data:");
            match v5_data.battery_voltage {
                Some(voltage) => {
                    println!(
                        "     🔋 Battery: {} mV ({:.2}V)",
                        voltage,
                        voltage as f64 / 1000.0
                    );

                    // Battery level estimation (rough)
                    let level = match voltage {
                        v if v >= 2900 => "High",
                        v if v >= 2600 => "Medium",
                        v if v >= 2400 => "Low",
                        _ => "Very Low",
                    };
                    println!("     📊 Battery level: {}", level);
                }
                None => println!("     🔋 Battery: Not available"),
            }

            match v5_data.tx_power {
                Some(power) => println!("     📡 TX Power: {} dBm", power),
                None => println!("     📡 TX Power: Not available"),
            }

            // Measurement info
            println!("   📈 Measurement Data:");
            match v5_data.measurement_sequence {
                Some(seq) => println!("     🔢 Sequence: {}", seq),
                None => println!("     🔢 Sequence: Not available"),
            }
        }
        RuuviData::V6(_) => {
            println!("   📊 Data Format V6 (not yet implemented)");
        }
        RuuviData::E1(_) => {
            println!("   📊 Data Format E1 (not yet implemented)");
        }
    }
    println!();
}

/// Demonstrate various error conditions
fn demonstrate_error_handling() {
    let a = format!("06{}", "00".repeat(23));
    let b = format!("05{}", "00".repeat(50));
    let test_cases = vec![
        ("", "Empty string"),
        ("XX", "Invalid hex characters"),
        ("0512FC", "Too short"),
        (&a, "Unsupported format (v6 not implemented)"),
        (&b, "Too long"),
    ];

    for (hex_data, description) in test_cases {
        println!("  Testing: {}", description);
        match decode(hex_data) {
            Ok(_) => println!("    ✅ Unexpectedly succeeded"),
            Err(e) => println!("    ❌ Expected error: {}", e),
        }
    }

    // Test BLE extraction errors
    println!("  Testing BLE extraction errors:");
    let invalid_ble_cases = vec![
        ("020106", "No Ruuvi manufacturer data"),
        ("GGHHII", "Invalid hex in BLE"),
    ];

    for (ble_data, description) in invalid_ble_cases {
        println!("    Testing: {}", description);
        match extract_ruuvi_from_ble(ble_data) {
            Some(payload) => println!("      ✅ Extracted: {}", payload),
            None => println!("      ❌ No Ruuvi data found (expected)"),
        }
    }
}
