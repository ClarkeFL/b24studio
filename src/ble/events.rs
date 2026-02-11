use std::collections::HashMap;
use uuid::Uuid;
use crate::ble::error::BleError;
use crate::protocol::types::DeviceAction;

/// Messages sent from the BLE worker back to the UI thread
#[derive(Debug)]
pub enum BleEvent {
    /// A device was discovered during scanning
    DeviceDiscovered {
        peripheral_id: String,
        name: Option<String>,
        rssi: Option<i16>,
        manufacturer_data: HashMap<u16, Vec<u8>>,
        service_uuids: Vec<Uuid>,
    },
    /// Scan completed or was stopped
    ScanStopped,
    /// Successfully connected and PIN authenticated
    Connected,
    /// Disconnected (intentional or lost connection)
    Disconnected {
        reason: Option<String>,
    },
    /// A characteristic was read successfully
    CharacteristicRead {
        uuid: Uuid,
        data: Vec<u8>,
    },
    /// A characteristic was written successfully
    CharacteristicWritten {
        uuid: Uuid,
    },
    /// A notification was received from a subscribed characteristic
    Notification {
        uuid: Uuid,
        data: Vec<u8>,
    },
    /// An advanced parameter was read
    AdvancedRead {
        index: u8,
        data: Vec<u8>,
    },
    /// An advanced parameter was written
    AdvancedWritten {
        index: u8,
    },
    /// A device action was executed
    ActionExecuted(DeviceAction),
    /// An error occurred
    Error(BleError),
}
