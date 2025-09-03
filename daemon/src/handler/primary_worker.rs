use crate::handler::messaging::DaemonMessage;
use crate::handler::pipewire::manager::{run_pipewire_manager, PipewireManagerConfig};
use crate::handler::primary_worker::ManagerMessage::{Execute, GetAudioConfiguration, SetMetering};
use crate::servers::http_server::{MeterEvent, PatchEvent};
use crate::stop::Stop;
use crate::APP_NAME_ID;
use anyhow::Context;
use anyhow::Result;
use json_patch::diff;
use log::{debug, error, info, warn};
use pipeweaver_ipc::commands::{
    APICommand, APICommandResponse, AudioConfiguration, DaemonCommand, DaemonResponse, DaemonStatus,
};
use pipeweaver_profile::Profile;
use std::fs;
use std::fs::{create_dir_all, File};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use tokio::sync::broadcast::Sender;
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use tokio::{select, task, time};

type Manage = mpsc::Sender<ManagerMessage>;

pub struct PrimaryWorker {
    last_status: DaemonStatus,
    patch_broadcast: Sender<PatchEvent>,
    meter_broadcast: Sender<MeterEvent>,

    shutdown: Stop,
}

impl PrimaryWorker {
    fn new(shutdown: Stop, patch: Sender<PatchEvent>, meter: Sender<MeterEvent>) -> Self {
        Self {
            last_status: DaemonStatus::default(),
            patch_broadcast: patch,
            meter_broadcast: meter,

            shutdown,
        }
    }

    async fn run(
        &mut self,
        mut message_receiver: mpsc::Receiver<DaemonMessage>,
        config_path: PathBuf,
    ) {
        let profile_path = config_path.join(format!("{}-profile.json", APP_NAME_ID));
        let mut first_run = true;


        'main: loop {
            if !first_run {
                // We need to wait a couple of seconds to make sure the teardown is complete
                info!("[PrimaryWorker] Restarting Pipewire Manager");
                sleep(Duration::from_secs(2)).await;
            } else {
                first_run = false;
            }

            info!("[PrimaryWorker] Starting Primary Worker");
            info!("[PrimaryWorker] Loading Profile");

            let profile = self.load_profile(&profile_path);

            // Used to pass messages into the Pipewire Manager
            let (command_sender, command_receiver) = mpsc::channel(32);
            let (worker_sender, mut worker_receiver) = mpsc::channel(32);
            let (stop_sender, stop_receiver) = oneshot::channel();
            let (ready_sender, ready_receiver) = oneshot::channel();
            let mut profile_tick = time::interval(Duration::from_secs(5));

            debug!("[PrimaryWorker] Spawning Pipewire Task..");
            let config = PipewireManagerConfig {
                profile,

                command_receiver,
                worker_sender,

                meter_sender: self.meter_broadcast.clone(),
                ready_sender: Some(ready_sender),
            };
            task::spawn(run_pipewire_manager(config, stop_sender));

            // Wait until the manager reports itself as ready
            let _ = ready_receiver.await;

            let mut profile_changed = false;

            loop {
                select! {
                    Some(message) = message_receiver.recv() => {
                        match self.handle_message(&command_sender, message).await {
                            MessageResult::UpdateState => {
                                self.update_status(&command_sender).await;
                                profile_changed = true;
                            }
                            MessageResult::Reset => {
                                // Restart the Pipewire Manager, so continue on the main loop
                                info!("[PrimaryWorker] Restarting Pipewire Manager");
                                break;
                            }
                            MessageResult::None => {}
                        }
                    }

                    Some(message) = worker_receiver.recv() => {
                        match message {
                            WorkerMessage::DevicesChanged => {
                                // A physical device has changed, we need to update the main
                                // status to include it.
                                self.update_status(&command_sender).await;
                            }
                            WorkerMessage::ProfileChanged => {
                                // Something's been changed in the Profile
                                self.update_status(&command_sender).await;
                                profile_changed = true;
                            }
                        }
                    }

                    _ = profile_tick.tick() => {
                        if profile_changed {
                            profile_changed = false;
                            let _ = self.save_profile(&profile_path, &self.last_status.audio.profile);
                        }
                    },

                    _ = self.shutdown.recv() => {
                        info!("[PrimaryWorker] Stopping");
                        info!("[PrimaryWorker] Stopping Pipewire Manager");
                        let _ = command_sender.send(ManagerMessage::Quit).await;

                        // Wait for the Stop message
                        let _ = stop_receiver.await;
                        break 'main;
                    }
                }
            }

            // Do a final profile save on shutdown
            let _ = self.save_profile(&profile_path, &self.last_status.audio.profile);
            info!("[PrimaryWorker] Stopped");
        }

        let _ = self.save_profile(&profile_path, &self.last_status.audio.profile);
        info!("[PrimaryWorker] Stopped");
    }

    async fn handle_message(&mut self, pw_tx: &Manage, message: DaemonMessage) -> MessageResult {
        let mut update = false;

        match message {
            DaemonMessage::GetStatus(tx) => {
                let _ = tx.send(self.last_status.clone());
            }
            DaemonMessage::RunDaemon(command, tx) => {
                match command {
                    DaemonCommand::SetMetering(enabled) => {
                        let _ = pw_tx.send(SetMetering(enabled)).await;
                    }
                    DaemonCommand::OpenInterface => {
                        // TODO: Need to pull in the HTTP config
                        // TODO: Attempt 'app' launch before browser

                        if let Err(e) = open::that("http://localhost:14565") {
                            warn!("Unable to open web interface: {}", e);
                        }
                    }
                    DaemonCommand::ResetAudio => {}
                }
                let _ = tx.send(DaemonResponse::Ok);
                update = true;
            }
            DaemonMessage::RunPipewire(command, response) => {
                let (tx, rx) = oneshot::channel();
                if let Err(e) = pw_tx.send(Execute(command, tx)).await {
                    let _ = response.send(APICommandResponse::Err(e.to_string()));
                    return MessageResult::None;
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
        if update {
            return MessageResult::UpdateState;
        }
        MessageResult::None
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

    fn load_profile(&self, path: &PathBuf) -> Profile {
        info!("[Profile] Loading");
        match File::open(path) {
            Ok(reader) => {
                let settings = serde_json::from_reader(reader);
                settings.unwrap_or_else(|e| {
                    warn!(
                        "[Profile] Found, but unable to Load ({}), sending default",
                        e
                    );
                    Profile::base_settings()
                })
            }
            Err(_) => {
                warn!("[Profile] Not Found, sending default");
                Profile::base_settings()
            }
        }
    }

    fn save_profile(&self, path: &PathBuf, profile: &Profile) -> Result<()> {
        info!("[Profile] Saving");

        if let Some(parent) = path.parent() {
            if let Err(e) = create_dir_all(parent) {
                if e.kind() != ErrorKind::AlreadyExists {
                    return Err(e).context(format!(
                        "Could not create config directory at {}",
                        parent.to_string_lossy()
                    ))?;
                }
            }
        }

        if path.exists() {
            fs::remove_file(path).context("Unable to remove old Profile")?;
        }

        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, profile)?;

        info!("[Profile] Saved");
        Ok(())
    }
}

pub enum MessageResult {
    UpdateState,
    Reset,
    None,
}

#[derive(Debug)]
pub enum ManagerMessage {
    Execute(APICommand, oneshot::Sender<APICommandResponse>),
    GetAudioConfiguration(oneshot::Sender<AudioConfiguration>),
    SetMetering(bool),
    Quit,
}

pub enum WorkerMessage {
    DevicesChanged,
    ProfileChanged,
}

pub async fn start_primary_worker(
    message_receiver: mpsc::Receiver<DaemonMessage>,
    shutdown: Stop,
    broadcast_tx: Sender<PatchEvent>,
    meter_tx: Sender<MeterEvent>,
    config_path: PathBuf,
) {
    let mut manager = PrimaryWorker::new(shutdown, broadcast_tx, meter_tx);
    manager.run(message_receiver, config_path).await;
}
