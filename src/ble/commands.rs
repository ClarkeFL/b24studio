use uuid::Uuid;
use crate::protocol::types::DeviceAction;

/// Messages sent from the UI thread to the BLE worker
#[derive(Debug)]
pub enum BleCommand {
    /// Start scanning for B24 devices
    StartScan,
    /// Stop scanning
    StopScan,
    /// Connect to a device and authenticate with the given PIN
    Connect {
        peripheral_id: String,
        config_pin: u32,
    },
    /// Disconnect from the current device
    Disconnect,
    /// Read a characteristic by UUID
    ReadCharacteristic(Uuid),
    /// Write data to a characteristic by UUID
    WriteCharacteristic {
        uuid: Uuid,
        data: Vec<u8>,
    },
    /// Subscribe to notifications on a characteristic
    Subscribe(Uuid),
    /// Unsubscribe from notifications on a characteristic
    Unsubscribe(Uuid),
    /// Read an advanced parameter: write index, then read data
    ReadAdvanced {
        index: u8,
    },
    /// Write an advanced parameter: write index, then write data
    WriteAdvanced {
        index: u8,
        data: Vec<u8>,
    },
    /// Execute a device action (tare, reboot, etc.)
    ExecuteAction(DeviceAction),
    /// Read All: read a batch of characteristics
    ReadAll(Vec<Uuid>),
    /// Shut down the BLE worker
    Shutdown,
}
