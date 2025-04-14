use crate::handler::messaging::DaemonMessage;
use crate::handler::pipewire::manager::run_pipewire_manager;
use crate::handler::primary_worker::ManagerMessage::{Execute, GetAudioConfiguration};
use crate::servers::http_server::PatchEvent;
use crate::stop::Stop;
use ipc::commands::{
    APICommand, APICommandResponse, AudioConfiguration, DaemonResponse, DaemonStatus,
};
use json_patch::diff;
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::broadcast::Sender;
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use tokio::{select, task};

type Manage = mpsc::Sender<ManagerMessage>;

pub struct PrimaryWorker {
    last_status: DaemonStatus,
    patch_broadcast: Sender<PatchEvent>,

    shutdown: Stop,
}

impl PrimaryWorker {
    fn new(shutdown: Stop, patch_broadcast: Sender<PatchEvent>) -> Self {
        Self {
            last_status: DaemonStatus::default(),
            patch_broadcast,

            shutdown,
        }
    }

    async fn run(&mut self, mut message_receiver: mpsc::Receiver<DaemonMessage>, config_path: PathBuf) {
        info!("[PrimaryWorker] Starting Primary Worker..");

        // Used to pass messages into the Pipewire Manager
        let (pipewire_sender, pipewire_receiver) = mpsc::channel(32);
        let (worker_sender, mut worker_receiver) = mpsc::channel(32);
        let (stop_sender, stop_receiver) = tokio::sync::oneshot::channel();

        debug!("[PrimaryWorker] Spawning Pipewire Task..");
        task::spawn(run_pipewire_manager(pipewire_receiver, worker_sender, stop_sender));

        // Until we're doing this properly...
        sleep(Duration::from_secs(3)).await;
        self.update_status(&pipewire_sender).await;

        loop {
            select! {
                Some(message) = message_receiver.recv() => {
                    if self.handle_message(&pipewire_sender, message).await {
                        self.update_status(&pipewire_sender).await;
                    }
                }

                Some(message) = worker_receiver.recv() => {
                    match message {
                        WorkerMessage::RefreshState => {
                            self.update_status(&pipewire_sender).await;
                        }
                    }
                }

                _ = self.shutdown.recv() => {
                    debug!("Shutdown Received!");
                    let _ = pipewire_sender.send(ManagerMessage::Quit).await;

                    // Wait for the Stop message
                    let _ = stop_receiver.await;
                    break;
                }
            }
        }

        info!("[PrimaryWorker] Stopped");
    }

    async fn handle_message(&mut self, pw_tx: &Manage, message: DaemonMessage) -> bool {
        let mut update = false;

        match message {
            DaemonMessage::GetStatus(tx) => {
                let _ = tx.send(self.last_status.clone());
            }
            DaemonMessage::RunDaemon(_command, tx) => {
                let _ = tx.send(DaemonResponse::Ok);
                update = true;
            }
            DaemonMessage::RunPipewire(command, response) => {
                let (tx, rx) = oneshot::channel();
                if let Err(e) = pw_tx.send(Execute(command, tx)).await {
                    let _ = response.send(APICommandResponse::Err(e.to_string()));
                    return false;
                }
                match rx.await {
                    Ok(command_response) => {
                        let _ = response.send(command_response);
                        update = true;
                    }
                    Err(e) => {
                        let _ = response.send(APICommandResponse::Err(e.to_string()));
                    }
                }
            }
        }
        update
    }

    async fn update_status(&mut self, pw_tx: &Manage) {
        let mut status = DaemonStatus::default();

        let (cmd_tx, cmd_rx) = oneshot::channel();

        if pw_tx.send(GetAudioConfiguration(cmd_tx)).await.is_err() {
            warn!("Unable to Send GetConfig Request");
            return;
        }

        let Ok(config) = cmd_rx.await else {
            error!("Unable to Obtain Audio Configuration from Pipewire Manager");
            return;
        };

        status.audio = config;

        let previous = serde_json::to_value(&self.last_status).unwrap();
        let new = serde_json::to_value(&status).unwrap();

        let patch = diff(&previous, &new);
        if !patch.0.is_empty() {
            // Something has changed in our config, broadcast it to listeners
            let _ = self.patch_broadcast.send(PatchEvent { data: patch });
        }

        self.last_status = status;
    }
}

#[derive(Debug)]
pub enum ManagerMessage {
    Execute(APICommand, oneshot::Sender<APICommandResponse>),
    GetAudioConfiguration(oneshot::Sender<AudioConfiguration>),
    Quit,
}

pub enum WorkerMessage {
    RefreshState,
}

pub async fn start_primary_worker(
    message_receiver: mpsc::Receiver<DaemonMessage>,
    shutdown: Stop,
    broadcast_tx: Sender<PatchEvent>,
    config_path: PathBuf,
) {
    let mut manager = PrimaryWorker::new(shutdown, broadcast_tx);
    manager.run(message_receiver, config_path).await;
}
