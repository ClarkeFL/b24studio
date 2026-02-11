use thiserror::Error;

#[derive(Error, Debug)]
pub enum BleError {
    #[error("No BLE adapter found")]
    NoAdapter,
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    #[error("Not connected to any device")]
    NotConnected,
    #[error("Characteristic not found: {0}")]
    CharacteristicNotFound(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Write failed: {0}")]
    WriteFailed(String),
    #[error("Read failed: {0}")]
    ReadFailed(String),
    #[error("PIN authentication timeout (5 seconds)")]
    PinTimeout,
    #[error("BLE error: {0}")]
    Other(String),
}
