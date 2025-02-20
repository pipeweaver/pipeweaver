use crate::handler::messaging::DaemonMessage;
use crate::handler::pipewire::manager::run_pipewire_manager;
use crate::servers::http_server::PatchEvent;
use crate::stop::Stop;
use log::{debug, info};
use pipecast_ipc::commands::{DaemonResponse, DaemonStatus};
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc;
use tokio::{select, task};

pub struct PrimaryWorker {
    last_status: DaemonStatus,
    patch_broadcast: Sender<PatchEvent>,

    /// Used for messages that the DeviceState may need updating
    update_sender: mpsc::Sender<()>,
    update_receiver: mpsc::Receiver<()>,

    shutdown: Stop,

}

impl PrimaryWorker {
    fn new(shutdown: Stop, patch_broadcast: Sender<PatchEvent>) -> Self {
        let (update_sender, update_receiver) = mpsc::channel(1);
        Self {
            last_status: DaemonStatus::default(),
            patch_broadcast,

            update_sender,
            update_receiver,

            shutdown,
        }
    }

    async fn run(&mut self, mut message_receiver: mpsc::Receiver<DaemonMessage>) {
        info!("[PrimaryWorker] Starting Primary Worker..");

        task::spawn(run_pipewire_manager());

        loop {
            select! {
                Some(message) = message_receiver.recv() => {
                    self.handle_message(message).await;
                }
                _ = self.shutdown.recv() => {
                    debug!("Shutdown Received!");
                    break;
                }
            }
        }

        info!("[PrimaryWorker] Stopped");
    }

    async fn handle_message(&mut self, message: DaemonMessage) {
        let mut update = false;

        match message {
            DaemonMessage::GetStatus(tx) => {
                let _ = tx.send(self.last_status.clone());
                update = true;
            }
            DaemonMessage::RunDaemon(_command, tx) => {
                let _ = tx.send(DaemonResponse::Ok);
                update = true;
            }
            DaemonMessage::RunPipewire(_, _) => {}
        }
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