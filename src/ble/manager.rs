use tokio::sync::mpsc;
use crate::ble::commands::BleCommand;
use crate::ble::events::BleEvent;
use crate::ble::worker::BleWorker;

/// Handle held by the UI to communicate with the BLE worker
pub struct BleHandle {
    pub cmd_tx: mpsc::UnboundedSender<BleCommand>,
    pub evt_rx: mpsc::UnboundedReceiver<BleEvent>,
}

impl BleHandle {
    pub fn send(&self, cmd: BleCommand) {
        let _ = self.cmd_tx.send(cmd);
    }
}

/// Spawns the BLE worker on a dedicated thread with its own tokio runtime.
/// Returns a BleHandle for the UI thread to send commands and receive events.
pub fn spawn_ble_worker() -> BleHandle {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (evt_tx, evt_rx) = mpsc::unbounded_channel();

    std::thread::Builder::new()
        .name("ble-worker".to_string())
        .spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async {
                let worker = BleWorker::new(cmd_rx, evt_tx);
                worker.run().await;
            });
        })
        .expect("Failed to spawn BLE worker thread");

    BleHandle { cmd_tx, evt_rx }
}
