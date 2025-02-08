use crate::handler::messaging::DaemonMessage;
use crate::servers::http_server::PatchEvent;
use crate::stop::Stop;
use log::{debug, info};
use tokio::select;
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc;

pub struct PrimaryWorker {
    shutdown: Stop,
    broadcast_tx: Sender<PatchEvent>,
}

impl PrimaryWorker {
    fn new(shutdown: Stop, broadcast_tx: Sender<PatchEvent>) -> Self {
        Self {
            shutdown,
            broadcast_tx,
        }
    }

    async fn run(&mut self, mut message_receiver: mpsc::Receiver<DaemonMessage>) {
        info!("[PrimaryWorker] Starting Primary Worker..");

        loop {
            select! {
                Some(message) = message_receiver.recv() => {
                    debug!("Received Message");
                }
                _ = self.shutdown.recv() => {
                    debug!("Shutdown Received!");
                    break;
                }
            }
        }

        info!("[PrimaryWorker] Stopped");
    }
}

pub async fn start_primary_worker(
    message_receiver: mpsc::Receiver<DaemonMessage>,
    shutdown: Stop,
    broadcast_tx: Sender<PatchEvent>,
) {
    let mut manager = PrimaryWorker::new(shutdown, broadcast_tx);
    manager.run(message_receiver).await;
}