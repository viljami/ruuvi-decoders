//! BLE Scanner Example for Ruuvi Decoders
//!
//! This example simulates a BLE scanner that processes Ruuvi advertisements
//! in real-time, demonstrating how to handle multiple sensors and data filtering.
//!
//! Run with: cargo run --example ble_scanner

use ruuvi_decoders::{RuuviData, decode, extract_ruuvi_from_ble};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Represents a BLE advertisement packet
#[derive(Debug, Clone)]
struct BleAdvertisement {
    mac_address: String,
    rssi: i16,
    timestamp: u64,
    raw_data: String,
}

/// Sensor information tracked by the scanner
#[derive(Debug, Clone)]
struct SensorInfo {
    last_seen: u64,
    packet_count: u32,
    last_temperature: Option<f64>,
    last_humidity: Option<f64>,
    last_rssi: i16,
    format: Option<String>,
}

/// Simple BLE scanner simulator
struct BleScanner {
    sensors: HashMap<String, SensorInfo>,
    total_packets: u32,
    ruuvi_packets: u32,
    start_time: u64,
}

impl BleScanner {
    fn new() -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            sensors: HashMap::new(),
            total_packets: 0,
            ruuvi_packets: 0,
            start_time,
        }
    }

    /// Process a BLE advertisement
    fn process_advertisement(&mut self, ad: BleAdvertisement) {
        self.total_packets += 1;

        // Try to extract Ruuvi data
        if let Some(ruuvi_hex) = extract_ruuvi_from_ble(&ad.raw_data) {
            self.ruuvi_packets += 1;

            match decode(&ruuvi_hex) {
                Ok(ruuvi_data) => {
                    self.update_sensor_info(&ad, &ruuvi_data);
                }
                Err(e) => {
                    println!(
                        "⚠️  Failed to decode Ruuvi data from {}: {}",
                        ad.mac_address, e
                    );
                }
            }
        }
    }

    /// Update sensor information with new data
    fn update_sensor_info(&mut self, ad: &BleAdvertisement, ruuvi_data: &RuuviData) {
        let sensor_mac = match ruuvi_data {
            RuuviData::V5(data_format_v5) => data_format_v5.mac_address.clone(),
            RuuviData::V6(data_format_v6) => data_format_v6.mac_address.clone(),
            RuuviData::E1(data_format_e1) => data_format_e1.mac_address.clone(),
        };

        let sensor_info = self.sensors.entry(sensor_mac.clone()).or_insert_with(|| {
            println!("🆕 New Ruuvi sensor discovered: {sensor_mac}");
            SensorInfo {
                last_seen: ad.timestamp,
                packet_count: 0,
                last_temperature: None,
                last_humidity: None,
                last_rssi: ad.rssi,
                format: None,
            }
        });

        // Update sensor info
        sensor_info.last_seen = ad.timestamp;
        sensor_info.packet_count += 1;
        sensor_info.last_rssi = ad.rssi;
        sensor_info.format = Some(format!("{ruuvi_data:?}"));

        // Extract sensor values based on format
        match ruuvi_data {
            RuuviData::V5(v5_data) => {
                println!("📊 V5 data received from {sensor_mac}: {v5_data:?}");
            }
            RuuviData::V6(v6_data) => {
                println!("📊 V6 data received from {sensor_mac}: {v6_data:?}");
            }
            RuuviData::E1(e1_data) => {
                println!("📊 E1 data received from {sensor_mac}: {e1_data:?}");
            }
        }
    }

    /// Print scanner statistics
    fn print_statistics(&self) {
        let runtime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - self.start_time;

        println!("\n📊 Scanner Statistics");
        println!("====================");
        println!("Runtime: {} seconds", runtime);
        println!("Total packets processed: {}", self.total_packets);
        println!("Ruuvi packets: {}", self.ruuvi_packets);
        println!("Unique sensors discovered: {}", self.sensors.len());

        if runtime > 0 {
            println!(
                "Packets per second: {:.1}",
                self.total_packets as f64 / runtime as f64
            );
        }
    }

    /// Print detailed sensor information
    fn print_sensor_details(&self) {
        if self.sensors.is_empty() {
            println!("\n📱 No Ruuvi sensors discovered yet");
            return;
        }

        println!("\n📱 Discovered Ruuvi Sensors");
        println!("===========================");

        for (mac, info) in &self.sensors {
            let age = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - info.last_seen;

            println!("\n🏷️  Sensor: {}", mac);
            println!("   Format: {}", info.format.as_deref().unwrap_or("Unknown"));
            println!("   Packets received: {}", info.packet_count);
            println!("   Last seen: {} seconds ago", age);
            println!("   RSSI: {} dBm", info.last_rssi);

            if let Some(temp) = info.last_temperature {
                println!("   Temperature: {:.2}°C", temp);
            }

            if let Some(humidity) = info.last_humidity {
                println!("   Humidity: {:.2}%", humidity);
            }

            // Connection quality assessment
            let quality = match info.last_rssi {
                rssi if rssi >= -50 => "Excellent",
                rssi if rssi >= -60 => "Good",
                rssi if rssi >= -70 => "Fair",
                _ => "Poor",
            };
            println!("   Signal quality: {} ({}dBm)", quality, info.last_rssi);
        }
    }
}

fn main() {
    println!("🔍 Ruuvi BLE Scanner Example");
    println!("=============================\n");

    let mut scanner = BleScanner::new();

    // Simulate BLE advertisements from multiple Ruuvi sensors
    let simulated_advertisements = generate_sample_advertisements();

    println!(
        "📡 Processing {} simulated BLE advertisements...\n",
        simulated_advertisements.len()
    );

    for ad in simulated_advertisements {
        scanner.process_advertisement(ad);

        // Simulate real-time processing delay
        std::thread::sleep(Duration::from_millis(10));
    }

    // Print results
    scanner.print_statistics();
    scanner.print_sensor_details();

    println!("\n🔄 Demonstrating continuous monitoring...");

    // Simulate ongoing monitoring for a few more seconds
    for i in 0..5 {
        println!("⏰ Monitoring... ({}/5)", i + 1);

        // Generate some additional packets
        let additional_ads = generate_followup_advertisements();
        for ad in additional_ads {
            scanner.process_advertisement(ad);
        }

        std::thread::sleep(Duration::from_secs(1));
    }

    scanner.print_statistics();
    println!("\n✅ BLE Scanner example completed!");
}

/// Generate sample BLE advertisements for testing
fn generate_sample_advertisements() -> Vec<BleAdvertisement> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    vec![
        // Sensor 1: Valid v5 data (from official test vector)
        BleAdvertisement {
            mac_address: "CB:B8:33:4C:88:4F".to_string(),
            rssi: -65,
            timestamp: now,
            raw_data: "02010603031691FF990405012FC5394C37C0004FFFC040CAC364200CDCBB8334C884F"
                .to_string(),
        },
        // Sensor 2: Maximum values test vector
        BleAdvertisement {
            mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
            rssi: -55,
            timestamp: now + 1,
            raw_data: "02010603031691FF99047FFFFFFEFFFE7FFF7FFF7FFFFFDEFEFFFECBB8334C884F"
                .to_string(),
        },
        // Sensor 3: Cold temperature
        BleAdvertisement {
            mac_address: "11:22:33:44:55:66".to_string(),
            rssi: -75,
            timestamp: now + 2,
            raw_data: "02010603031691FF9904058001000000008001800180010000000000112233445566"
                .to_string(),
        },
        // Non-Ruuvi advertisement (should be ignored)
        BleAdvertisement {
            mac_address: "99:88:77:66:55:44".to_string(),
            rssi: -60,
            timestamp: now + 3,
            raw_data: "020106030316910255AA".to_string(),
        },
        // Sensor 4: Hot temperature
        BleAdvertisement {
            mac_address: "DD:EE:FF:AA:BB:CC".to_string(),
            rssi: -50,
            timestamp: now + 4,
            raw_data: "02010603031691FF99040519C47C025A8BC4A53C00FB00000000E7FEDEEFFAABBCC"
                .to_string(),
        },
    ]
}

/// Generate follow-up advertisements to simulate ongoing monitoring
fn generate_followup_advertisements() -> Vec<BleAdvertisement> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    vec![
        // Update from Sensor 1 with slightly different temperature
        BleAdvertisement {
            mac_address: "CB:B8:33:4C:88:4F".to_string(),
            rssi: -63, // Signal got slightly better
            timestamp: now,
            raw_data: "02010603031691FF990405013C5394C37C0004FFFC040CAC364201CDCBB8334C884F"
                .to_string(),
        },
        // Update from Sensor 2
        BleAdvertisement {
            mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
            rssi: -58,
            timestamp: now,
            raw_data: "02010603031691FF990405157C025A8BC4A53C00FB00000000E7FEAABBCCDDEEFF"
                .to_string(),
        },
    ]
}
