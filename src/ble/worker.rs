use tokio::sync::mpsc;
use btleplug::api::{
    Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter, WriteType,
    Characteristic,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::stream::StreamExt;
use uuid::Uuid;
use log::{info, warn, error, debug};

use crate::ble::commands::BleCommand;
use crate::ble::events::BleEvent;
use crate::ble::error::BleError;
use crate::protocol::uuids;
use crate::protocol::codec;

pub struct BleWorker {
    cmd_rx: mpsc::UnboundedReceiver<BleCommand>,
    evt_tx: mpsc::UnboundedSender<BleEvent>,
    adapter: Option<Adapter>,
    peripheral: Option<Peripheral>,
    characteristics: Vec<Characteristic>,
}

impl BleWorker {
    pub fn new(
        cmd_rx: mpsc::UnboundedReceiver<BleCommand>,
        evt_tx: mpsc::UnboundedSender<BleEvent>,
    ) -> Self {
        Self {
            cmd_rx,
            evt_tx,
            adapter: None,
            peripheral: None,
            characteristics: Vec::new(),
        }
    }

    pub async fn run(mut self) {
        // Initialize BLE
        match Manager::new().await {
            Ok(manager) => {
                match manager.adapters().await {
                    Ok(adapters) => {
                        self.adapter = adapters.into_iter().next();
                        if self.adapter.is_none() {
                            let _ = self.evt_tx.send(BleEvent::Error(BleError::NoAdapter));
                        }
                    }
                    Err(e) => {
                        let _ = self.evt_tx.send(BleEvent::Error(BleError::Other(
                            format!("Failed to get adapters: {e}"),
                        )));
                    }
                }
            }
            Err(e) => {
                let _ = self.evt_tx.send(BleEvent::Error(BleError::Other(
                    format!("Failed to create BLE manager: {e}"),
                )));
            }
        }

        loop {
            match self.cmd_rx.recv().await {
                Some(BleCommand::Shutdown) | None => {
                    info!("BLE worker shutting down");
                    break;
                }
                Some(cmd) => self.handle_command(cmd).await,
            }
        }
    }

    async fn handle_command(&mut self, cmd: BleCommand) {
        match cmd {
            BleCommand::StartScan => self.start_scan().await,
            BleCommand::StopScan => self.stop_scan().await,
            BleCommand::Connect { peripheral_id, config_pin } => {
                self.connect(&peripheral_id, config_pin).await;
            }
            BleCommand::Disconnect => self.disconnect().await,
            BleCommand::ReadCharacteristic(uuid) => self.read_characteristic(uuid).await,
            BleCommand::WriteCharacteristic { uuid, data } => {
                self.write_characteristic(uuid, &data).await;
            }
            BleCommand::Subscribe(uuid) => self.subscribe(uuid).await,
            BleCommand::Unsubscribe(uuid) => self.unsubscribe(uuid).await,
            BleCommand::ReadAdvanced { index } => self.read_advanced(index).await,
            BleCommand::WriteAdvanced { index, data } => {
                self.write_advanced(index, &data).await;
            }
            BleCommand::ExecuteAction(action) => self.execute_action(action).await,
            BleCommand::ReadAll(uuids) => {
                for uuid in uuids {
                    self.read_characteristic(uuid).await;
                }
            }
            BleCommand::Shutdown => {} // handled in run loop
        }
    }

    async fn start_scan(&mut self) {
        let Some(adapter) = &self.adapter else {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::NoAdapter));
            return;
        };

        // Start scanning
        if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::Other(
                format!("Scan start failed: {e}"),
            )));
            return;
        }

        info!("BLE scan started");

        // Spawn a task to listen for discovery events
        let evt_tx = self.evt_tx.clone();
        let adapter_clone = adapter.clone();
        tokio::spawn(async move {
            let mut events = match adapter_clone.events().await {
                Ok(e) => e,
                Err(e) => {
                    error!("Failed to get adapter events: {e}");
                    return;
                }
            };

            while let Some(event) = events.next().await {
                match event {
                    CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                        if let Ok(peripheral) = adapter_clone.peripheral(&id).await {
                            let props = peripheral.properties().await.ok().flatten();
                            let name = props.as_ref().and_then(|p| p.local_name.clone());
                            let rssi = props.as_ref().and_then(|p| p.rssi);
                            let manufacturer_data = props.as_ref()
                                .map(|p| p.manufacturer_data.clone())
                                .unwrap_or_default();
                            let service_uuids = props.as_ref()
                                .map(|p| p.services.clone())
                                .unwrap_or_default();
                            let _ = evt_tx.send(BleEvent::DeviceDiscovered {
                                peripheral_id: id.to_string(),
                                name,
                                rssi,
                                manufacturer_data,
                                service_uuids,
                            });
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    async fn stop_scan(&mut self) {
        if let Some(adapter) = &self.adapter {
            let _ = adapter.stop_scan().await;
            let _ = self.evt_tx.send(BleEvent::ScanStopped);
            info!("BLE scan stopped");
        }
    }

    async fn connect(&mut self, peripheral_id: &str, config_pin: u32) {
        let Some(adapter) = &self.adapter else {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::NoAdapter));
            return;
        };

        // Stop scanning first
        let _ = adapter.stop_scan().await;

        // Find the peripheral
        let peripherals = match adapter.peripherals().await {
            Ok(p) => p,
            Err(e) => {
                let _ = self.evt_tx.send(BleEvent::Error(BleError::ConnectionFailed(
                    format!("Failed to list peripherals: {e}"),
                )));
                return;
            }
        };

        let peripheral = peripherals
            .into_iter()
            .find(|p| p.id().to_string() == peripheral_id);

        let Some(peripheral) = peripheral else {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::DeviceNotFound(
                peripheral_id.to_string(),
            )));
            return;
        };

        // Connect
        info!("Connecting to {peripheral_id}...");
        if let Err(e) = peripheral.connect().await {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::ConnectionFailed(
                format!("Connect failed: {e}"),
            )));
            return;
        }

        // Discover services
        info!("Discovering services...");
        if let Err(e) = peripheral.discover_services().await {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::ConnectionFailed(
                format!("Service discovery failed: {e}"),
            )));
            let _ = peripheral.disconnect().await;
            return;
        }

        self.characteristics = peripheral.characteristics().into_iter().collect();
        debug!("Found {} characteristics", self.characteristics.len());

        // Immediately write Configuration PIN (must be within 5 seconds)
        let pin_uuid = uuids::char_config_pin();
        let pin_data = codec::encode_u32_be(config_pin);
        if let Some(char) = self.find_characteristic(pin_uuid) {
            info!("Writing Configuration PIN...");
            if let Err(e) = peripheral.write(&char, &pin_data, WriteType::WithResponse).await {
                let _ = self.evt_tx.send(BleEvent::Error(BleError::WriteFailed(
                    format!("PIN write failed: {e}"),
                )));
                let _ = peripheral.disconnect().await;
                return;
            }
        } else {
            // If the Config PIN characteristic doesn't exist, the device is not a valid B24
            // or the PIN is wrong and the device is not exposing characteristics
            let _ = self.evt_tx.send(BleEvent::Error(BleError::ConnectionFailed(
                "Could not connect — check Configuration PIN is correct".to_string(),
            )));
            let _ = peripheral.disconnect().await;
            return;
        }

        // Verify that B24 characteristics are accessible after PIN write.
        // Re-discover services to pick up any newly-exposed characteristics.
        if let Err(e) = peripheral.discover_services().await {
            debug!("Re-discovery after PIN failed: {e}");
        }
        self.characteristics = peripheral.characteristics().into_iter().collect();

        // Check for a key B24 characteristic (data_rate) to verify PIN was accepted
        let verify_uuid = uuids::char_data_rate();
        if self.find_characteristic(verify_uuid).is_none() {
            warn!("B24 characteristics not accessible after PIN write — wrong PIN?");
            let _ = self.evt_tx.send(BleEvent::Error(BleError::ConnectionFailed(
                "Could not connect — check Configuration PIN is correct".to_string(),
            )));
            let _ = peripheral.disconnect().await;
            self.characteristics.clear();
            return;
        }

        // Subscribe to notifications for live data
        let notify_uuids = [
            uuids::char_status(),
            uuids::char_data_value(),
            uuids::char_data_units(),
        ];
        for uuid in &notify_uuids {
            if let Some(char) = self.find_characteristic(*uuid) {
                if let Err(e) = peripheral.subscribe(&char).await {
                    debug!("Failed to subscribe to {uuid}: {e}");
                }
            }
        }

        // Spawn notification listener
        let evt_tx = self.evt_tx.clone();
        let peripheral_clone = peripheral.clone();
        tokio::spawn(async move {
            let mut stream = match peripheral_clone.notifications().await {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to get notification stream: {e}");
                    return;
                }
            };
            while let Some(notification) = stream.next().await {
                let _ = evt_tx.send(BleEvent::Notification {
                    uuid: notification.uuid,
                    data: notification.value,
                });
            }
            // Stream ended = disconnected
            let _ = evt_tx.send(BleEvent::Disconnected {
                reason: Some("Notification stream ended".to_string()),
            });
        });

        self.peripheral = Some(peripheral);
        let _ = self.evt_tx.send(BleEvent::Connected);
        info!("Connected and authenticated successfully");
    }

    async fn disconnect(&mut self) {
        if let Some(peripheral) = self.peripheral.take() {
            let _ = peripheral.disconnect().await;
            self.characteristics.clear();
            let _ = self.evt_tx.send(BleEvent::Disconnected { reason: None });
            info!("Disconnected");
        }
    }

    async fn read_characteristic(&mut self, uuid: Uuid) {
        let Some(peripheral) = &self.peripheral else {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::NotConnected));
            return;
        };

        let Some(char) = self.find_characteristic(uuid) else {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::CharacteristicNotFound(
                uuid.to_string(),
            )));
            return;
        };

        match peripheral.read(&char).await {
            Ok(data) => {
                let _ = self.evt_tx.send(BleEvent::CharacteristicRead { uuid, data });
            }
            Err(e) => {
                let _ = self.evt_tx.send(BleEvent::Error(BleError::ReadFailed(
                    format!("{uuid}: {e}"),
                )));
            }
        }
    }

    async fn write_characteristic(&mut self, uuid: Uuid, data: &[u8]) {
        let Some(peripheral) = &self.peripheral else {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::NotConnected));
            return;
        };

        let Some(char) = self.find_characteristic(uuid) else {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::CharacteristicNotFound(
                uuid.to_string(),
            )));
            return;
        };

        match peripheral.write(&char, data, WriteType::WithResponse).await {
            Ok(()) => {
                let _ = self.evt_tx.send(BleEvent::CharacteristicWritten { uuid });
            }
            Err(e) => {
                let _ = self.evt_tx.send(BleEvent::Error(BleError::WriteFailed(
                    format!("{uuid}: {e}"),
                )));
            }
        }
    }

    async fn subscribe(&mut self, uuid: Uuid) {
        let Some(peripheral) = &self.peripheral else {
            let _ = self.evt_tx.send(BleEvent::Error(BleError::NotConnected));
            return;
        };

        if let Some(char) = self.find_characteristic(uuid) {
            if let Err(e) = peripheral.subscribe(&char).await {
                let _ = self.evt_tx.send(BleEvent::Error(BleError::Other(
                    format!("Subscribe failed for {uuid}: {e}"),
                )));
            }
        }
    }

    async fn unsubscribe(&mut self, uuid: Uuid) {
        let Some(peripheral) = &self.peripheral else {
            return;
        };

        if let Some(char) = self.find_characteristic(uuid) {
            let _ = peripheral.unsubscribe(&char).await;
        }
    }

    async fn read_advanced(&mut self, index: u8) {
        // Write index to ADV_INDEX characteristic
        let idx_data = codec::encode_u8(index);
        self.write_characteristic(uuids::char_adv_index(), &idx_data).await;

        // Read ADV_DATA characteristic
        let Some(peripheral) = &self.peripheral else { return; };
        let Some(char) = self.find_characteristic(uuids::char_adv_data()) else { return; };

        match peripheral.read(&char).await {
            Ok(data) => {
                let _ = self.evt_tx.send(BleEvent::AdvancedRead { index, data });
            }
            Err(e) => {
                let _ = self.evt_tx.send(BleEvent::Error(BleError::ReadFailed(
                    format!("Advanced[{index}]: {e}"),
                )));
            }
        }
    }

    async fn write_advanced(&mut self, index: u8, data: &[u8]) {
        // Write index to ADV_INDEX
        let idx_data = codec::encode_u8(index);
        self.write_characteristic(uuids::char_adv_index(), &idx_data).await;

        // Write data to ADV_DATA
        let Some(peripheral) = &self.peripheral else { return; };
        let Some(char) = self.find_characteristic(uuids::char_adv_data()) else { return; };

        match peripheral.write(&char, data, WriteType::WithResponse).await {
            Ok(()) => {
                let _ = self.evt_tx.send(BleEvent::AdvancedWritten { index });
            }
            Err(e) => {
                let _ = self.evt_tx.send(BleEvent::Error(BleError::WriteFailed(
                    format!("Advanced[{index}]: {e}"),
                )));
            }
        }
    }

    async fn execute_action(&mut self, action: crate::protocol::types::DeviceAction) {
        let (index, data) = action.command();
        self.write_advanced(index, &data).await;
        let _ = self.evt_tx.send(BleEvent::ActionExecuted(action));
    }

    fn find_characteristic(&self, uuid: Uuid) -> Option<Characteristic> {
        self.characteristics.iter().find(|c| c.uuid == uuid).cloned()
    }
}
