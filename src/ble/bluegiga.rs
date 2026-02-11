use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use log::{info, error, debug};
use tokio::sync::mpsc;

use crate::ble::events::BleEvent;

const BLED112_VID: u16 = 0x2458;
const BLED112_PID: u16 = 0x0001;

// BGAPI v1.x constants
const BGAPI_EVENT: u8 = 0x80;
const CLASS_SYSTEM: u8 = 0x00;
const CLASS_GAP: u8 = 0x06;
const EVT_SYSTEM_BOOT: u8 = 0x00;
const EVT_GAP_SCAN_RESPONSE: u8 = 0x00;
const AD_TYPE_MANUFACTURER_DATA: u8 = 0xFF;
const AD_TYPE_COMPLETE_NAME: u8 = 0x09;
const AD_TYPE_SHORT_NAME: u8 = 0x08;

/// Detect a BLED112 dongle by USB VID:PID. Returns the COM port name if found.
pub fn detect() -> Option<String> {
    let ports = serialport::available_ports().ok()?;
    for port in ports {
        if let serialport::SerialPortType::UsbPort(info) = &port.port_type {
            if info.vid == BLED112_VID && info.pid == BLED112_PID {
                return Some(port.port_name);
            }
        }
    }
    None
}

/// Run the BlueGiga scanner on the current thread (blocking).
/// Sends BleEvent::DeviceDiscovered and BleEvent::ManufacturerDataUpdate
/// through the shared event channel.
pub fn run(
    port_name: String,
    evt_tx: mpsc::UnboundedSender<BleEvent>,
    stop: Arc<AtomicBool>,
) {
    if let Err(e) = run_inner(&port_name, &evt_tx, &stop) {
        error!("BlueGiga scanner error: {e}");
    }
    info!("BlueGiga scanner stopped");
}

fn run_inner(
    port_name: &str,
    evt_tx: &mpsc::UnboundedSender<BleEvent>,
    stop: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Opening BlueGiga BLED112 on {port_name}");

    let mut port = serialport::new(port_name, 115200)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(100))
        .open()?;

    // Drain any stale buffered data
    let mut drain = [0u8; 256];
    loop {
        match port.read(&mut drain) {
            Ok(0) => break,
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(_) => break,
            _ => continue,
        }
    }

    // Stop any ongoing procedure first (in case dongle was left scanning)
    info!("Stopping any previous BLED112 procedure...");
    let _ = send_command(&mut *port, CLASS_GAP, 0x04, &[]); // gap_end_procedure
    std::thread::sleep(Duration::from_millis(100));

    // Drain responses from end_procedure
    loop {
        match port.read(&mut drain) {
            Ok(0) => break,
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(_) => break,
            _ => continue,
        }
    }

    // Set scan parameters: interval=200 (125ms), window=200 (125ms), passive
    info!("Configuring scan parameters...");
    send_command(
        &mut *port,
        CLASS_GAP,
        0x07, // gap_set_scan_parameters
        &[0xC8, 0x00, 0xC8, 0x00, 0x00], // interval=200 LE, window=200 LE, passive
    )?;
    // Read response
    wait_response(&mut *port, CLASS_GAP, 0x07, Duration::from_secs(1))?;

    // Start scanning: gap_discover(observation=0x02)
    info!("Starting BLE scan via BLED112...");
    send_command(
        &mut *port,
        CLASS_GAP,
        0x02, // gap_discover
        &[0x02], // observation mode
    )?;
    wait_response(&mut *port, CLASS_GAP, 0x02, Duration::from_secs(1))?;

    info!("BLED112 scanning active");

    // Track known devices for DeviceDiscovered vs update
    let mut known_devices: HashMap<String, String> = HashMap::new(); // mac -> name

    // Main scan loop
    loop {
        if stop.load(Ordering::Relaxed) {
            // Stop scanning
            let _ = send_command(&mut *port, CLASS_GAP, 0x04, &[]); // gap_end_procedure
            break;
        }

        match read_packet(&mut *port) {
            Ok(Some((hdr, payload))) => {
                // Check for gap_scan_response event
                if hdr[0] & 0x80 != 0
                    && hdr[2] == CLASS_GAP
                    && hdr[3] == EVT_GAP_SCAN_RESPONSE
                {
                    if let Some(scan) = parse_scan_response(&payload) {
                        let is_new = !known_devices.contains_key(&scan.mac);
                        if is_new {
                            known_devices.insert(scan.mac.clone(), scan.name.clone());
                        }

                        // Always send DeviceDiscovered (UI deduplicates)
                        let _ = evt_tx.send(BleEvent::DeviceDiscovered {
                            peripheral_id: scan.mac.clone(),
                            name: if scan.name.is_empty() {
                                None
                            } else {
                                Some(scan.name.clone())
                            },
                            rssi: Some(scan.rssi),
                            manufacturer_data: scan.manufacturer_data.clone(),
                            service_uuids: Vec::new(),
                        });

                        // Send ManufacturerDataUpdate for fast view mode updates
                        if !scan.manufacturer_data.is_empty() {
                            let _ = evt_tx.send(BleEvent::ManufacturerDataUpdate {
                                peripheral_id: scan.mac,
                                manufacturer_data: scan.manufacturer_data,
                            });
                        }
                    }
                }
            }
            Ok(None) => {} // timeout, no data
            Err(e) => {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                debug!("BlueGiga read error: {e}");
            }
        }
    }

    // Explicitly close the serial port before returning
    drop(port);
    info!("BlueGiga COM port closed");

    Ok(())
}

// ── BGAPI packet I/O ──────────────────────────────────────────────

fn send_command(
    port: &mut dyn serialport::SerialPort,
    class: u8,
    cmd: u8,
    payload: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let header = [0x00, payload.len() as u8, class, cmd];
    port.write_all(&header)?;
    if !payload.is_empty() {
        port.write_all(payload)?;
    }
    port.flush()?;
    Ok(())
}

fn read_packet(
    port: &mut dyn serialport::SerialPort,
) -> Result<Option<([u8; 4], Vec<u8>)>, Box<dyn std::error::Error>> {
    let mut header = [0u8; 4];
    match port.read_exact(&mut header) {
        Ok(()) => {}
        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => return Ok(None),
        Err(e) => return Err(e.into()),
    }

    let payload_len = (((header[0] & 0x07) as usize) << 8) | (header[1] as usize);
    let mut payload = vec![0u8; payload_len];
    if payload_len > 0 {
        port.read_exact(&mut payload)?;
    }

    Ok(Some((header, payload)))
}

fn wait_response(
    port: &mut dyn serialport::SerialPort,
    expected_class: u8,
    expected_cmd: u8,
    timeout: Duration,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            return Err(format!(
                "Timeout waiting for response class={expected_class:02X} cmd={expected_cmd:02X}"
            )
            .into());
        }
        match read_packet(port)? {
            Some((hdr, payload)) => {
                // Response: bit 7 = 0, class and cmd match
                if hdr[0] & 0x80 == 0 && hdr[2] == expected_class && hdr[3] == expected_cmd {
                    return Ok(payload);
                }
                // Otherwise it's an event or different response, skip
            }
            None => continue,
        }
    }
}

// ── Scan response parsing ─────────────────────────────────────────

struct ScanResult {
    rssi: i16,
    mac: String,
    name: String,
    manufacturer_data: HashMap<u16, Vec<u8>>,
}

fn parse_scan_response(payload: &[u8]) -> Option<ScanResult> {
    // Minimum: rssi(1) + ptype(1) + addr(6) + atype(1) + bond(1) + dlen(1) = 11
    if payload.len() < 11 {
        return None;
    }

    let rssi = payload[0] as i8 as i16;
    // payload[1] = packet_type
    // payload[2..8] = address (6 bytes, little-endian)
    let addr = &payload[2..8];
    let mac = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        addr[5], addr[4], addr[3], addr[2], addr[1], addr[0]
    );
    // payload[8] = address_type
    // payload[9] = bond
    let data_len = payload[10] as usize;

    if payload.len() < 11 + data_len {
        return None;
    }

    let ad_data = &payload[11..11 + data_len];
    let (manufacturer_data, name) = parse_ad_structures(ad_data);

    Some(ScanResult {
        rssi,
        mac,
        name,
        manufacturer_data,
    })
}

fn parse_ad_structures(data: &[u8]) -> (HashMap<u16, Vec<u8>>, String) {
    let mut manufacturer_data = HashMap::new();
    let mut name = String::new();
    let mut offset = 0;

    while offset < data.len() {
        let len = data[offset] as usize;
        if len == 0 || offset + 1 + len > data.len() {
            break;
        }

        let ad_type = data[offset + 1];
        let ad_payload = &data[offset + 2..offset + 1 + len];

        match ad_type {
            AD_TYPE_MANUFACTURER_DATA if ad_payload.len() >= 2 => {
                let company_id = u16::from_le_bytes([ad_payload[0], ad_payload[1]]);
                manufacturer_data.insert(company_id, ad_payload[2..].to_vec());
            }
            AD_TYPE_COMPLETE_NAME | AD_TYPE_SHORT_NAME => {
                if let Ok(n) = std::str::from_utf8(ad_payload) {
                    name = n.to_string();
                }
            }
            _ => {}
        }

        offset += 1 + len;
    }

    (manufacturer_data, name)
}
