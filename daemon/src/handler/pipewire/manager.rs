use crate::handler::pipewire::components::load_profile::LoadProfile;
use crate::handler::pipewire::components::physical::PhysicalDevices;
use crate::handler::pipewire::ipc::ipc::IPCHandler;
use crate::handler::primary_worker::{ManagerMessage, WorkerMessage};
use enum_map::EnumMap;
use log::{debug, error, info};
use pipeweaver_ipc::commands::{APICommandResponse, AudioConfiguration, PhysicalDevice};
use pipeweaver_pipewire::{MediaClass, PipewireMessage, PipewireNode, PipewireReceiver, PipewireRunner};
use pipeweaver_profile::Profile;
use pipeweaver_shared::{DeviceType, Mix};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tokio::select;
use tokio::sync::{mpsc, oneshot};

use tokio::task::JoinHandle;
use tokio::time::sleep;
use ulid::Ulid;

type StdRecv = std::sync::mpsc::Receiver<PipewireReceiver>;

pub(crate) struct PipewireManager {
    command_receiver: mpsc::Receiver<ManagerMessage>,
    worker_sender: mpsc::Sender<WorkerMessage>,
    ready_sender: Option<oneshot::Sender<()>>,

    pub(crate) pipewire: Option<PipewireRunner>,

    pub(crate) profile: Profile,
    pub(crate) source_map: HashMap<Ulid, EnumMap<Mix, Ulid>>,
    pub(crate) target_map: HashMap<Ulid, Ulid>,

    // Maps the connection of a PassThrough filter to a Physical Source id
    pub(crate) physical_source: HashMap<Ulid, Vec<u32>>,
    pub(crate) physical_target: HashMap<Ulid, Vec<u32>>,

    // A list of physical nodes
    pub(crate) node_list: EnumMap<DeviceType, Vec<PhysicalDevice>>,
    pub(crate) physical_nodes: HashMap<u32, PipewireNode>,
}

impl PipewireManager {
    pub fn new(config: PipewireManagerConfig) -> Self {
        Self {
            command_receiver: config.command_receiver,
            worker_sender: config.worker_sender,
            ready_sender: config.ready_sender,

            pipewire: None,

            profile: Profile::base_settings(),
            //profile: Default::default(),

            source_map: HashMap::default(),
            target_map: HashMap::default(),

            physical_source: HashMap::default(),
            physical_target: HashMap::default(),

            node_list: Default::default(),
            physical_nodes: Default::default(),
        }
    }

    pub(crate) fn pipewire(&self) -> &PipewireRunner {
        if let Some(pipewire) = &self.pipewire {
            return pipewire;
        }
        panic!("Attempted to Get Pipewire before starting");
    }

    async fn get_audio_config(&self) -> AudioConfiguration {
        AudioConfiguration {
            profile: self.profile.clone(),
            devices: self.node_list.clone(),
        }
    }

    pub async fn run(&mut self) {
        debug!("[Pipewire Runner] Starting Event Loop");
        let (send, recv) = std::sync::mpsc::channel();
        let send_sync = send.clone();

        // We need a largish buffer here because it's impossible to know how many devices pipewire
        // is going to throw at us at any given time, and they're all processed sequentially.
        // They'll also be sent while we're blocked loading the profile, so this needs to be
        // large enough to accommodate them until the profile has finished loading.
        let (send_async, mut recv_async) = mpsc::channel(256);

        // Spawn up the Sync -> Async task loop
        let receiver = thread::spawn(|| run_receiver_wrapper(recv, send_async));

        // Run up the Pipewire Handler
        self.pipewire = Some(PipewireRunner::new(send.clone()).unwrap());

        debug!("Loading Profile");
        if let Err(e) = self.load_profile().await {
            error!("Error Loading Profile: {}", e);
        }


        // So these are small timers which are set up when a new device is sent to us. Rather than
        // immediately processing, we wait half a second to make sure the device doesn't disappear
        // again as it's layout is being calculated, if this timer completes we should be safe to
        // assume the device configuration has stabilised.
        let mut device_timers: HashMap<u32, JoinHandle<()>> = HashMap::new();
        let (device_ready_tx, mut device_ready_rx) = mpsc::channel(256);

        // A simple list of devices which have been discovered, but not flagged 'Ready'
        let mut discovered_devices: HashMap<u32, PipewireNode> = HashMap::new();

        // Let the primary worker know we're ready
        let _ = self.ready_sender.take().expect("Ready Sender Missing").send(());

        loop {
            select!(
                Some(command) = self.command_receiver.recv() => {
                    match command {
                        ManagerMessage::Execute(command, tx) => {
                            let result = self.handle_command(command).await;

                            // Map the result to a PW Response and send it
                            let _ = tx.send(match result {
                                Ok(response) => response,
                                Err(e) => APICommandResponse::Err(e.to_string())
                            });
                        }
                        ManagerMessage::GetAudioConfiguration(tx) => {
                            let _ = tx.send(self.get_audio_config().await);
                        }
                        ManagerMessage::Quit => {
                            info!("[Manager] Stopping");
                            break;
                        }
                    }
                }
                Some(msg) = recv_async.recv() => {
                    match msg {
                        PipewireReceiver::DeviceAdded(node) => {
                            // Only do this if we don't already have a timer
                            if device_timers.contains_key(&node.node_id) {
                                continue;
                            }

                            // We need the name, and a message callback for when we're done
                            let done = device_ready_tx.clone();

                            // Spawn up a simple task that simply waits a second
                            let handle = tokio::spawn(async move {
                                sleep(Duration::from_millis(500)).await;
                                let _ = done.send(node.node_id).await;
                            });
                            device_timers.insert(node.node_id, handle);
                            discovered_devices.insert(node.node_id, node);
                        }
                        PipewireReceiver::DeviceRemoved(id) => {
                            if let Some(handle) = device_timers.remove(&id) {
                                debug!("Device Disappeared During Grace Period: {}", id);
                                discovered_devices.remove(&id);
                                handle.abort();
                            } else {
                                debug!("Natural Device Removal: {}", id);
                                if let Some(node) = self.physical_nodes.remove(&id) {
                                    match node.node_class {
                                        MediaClass::Source => {
                                            let _ = self.source_device_removed(id).await;
                                        }
                                        MediaClass::Sink => {
                                            let _ = self.target_device_removed(id).await;
                                        }
                                        MediaClass::Duplex => {
                                            let _ = self.source_device_removed(id).await;
                                            let _ = self.target_device_removed(id).await;
                                        }
                                    }
                                    let _ = self.worker_sender.send(WorkerMessage::ProfileChanged).await;
                                }
                            }
                        }
                        PipewireReceiver::ManagedLinkDropped(source, target) => {
                            debug!("Managed Link Removed: {:?} {:?}", source, target);
                        }
                        _ => {}
                    }
                }
                Some(node_id) = device_ready_rx.recv() => {
                    // A device has been sat here for 500ms without being removed
                    if let Some(device) = discovered_devices.remove(&node_id) {

                        debug!("Device Found: {:?}, Type: {:?}", device.description, device.node_class);
                        device_timers.remove(&node_id);

                        // Create the 'Status' object
                        let node = PhysicalDevice {
                            node_id: device.node_id,
                            name: device.name.clone(),
                            description: device.description.clone()
                        };
                        match device.node_class {
                            MediaClass::Source => {
                                let _ = self.source_device_added(node).await;
                            }
                            MediaClass::Sink => {
                                let _ = self.target_device_added(node).await;
                            }
                            MediaClass::Duplex => {
                                let _ = self.source_device_added(node.clone()).await;
                                let _ = self.target_device_added(node).await;
                            }
                        }

                        // Add node to our definitive list
                        self.physical_nodes.insert(device.node_id, device);
                        let _ = self.worker_sender.send(WorkerMessage::ProfileChanged).await;
                    } else {
                        panic!("Got a Timer Ready for non-existent Node");
                    }
                }
            );
        }
        info!("[Manager] Stopping Pipewire");
        let _ = self.pipewire().send_message(PipewireMessage::Quit);
        let runtime = self.pipewire.take();
        drop(runtime);

        info!("[Manager] Stopping Message Wrapper");
        let _ = send_sync.send(PipewireReceiver::Quit);
        let _ = receiver.join();


        info!("[Manager] Stopped");
    }
}

// Kinda ugly, but we're going to wrap around a blocking receiver, and bounce messages to an async
pub fn run_receiver_wrapper(recv: StdRecv, resend: mpsc::Sender<PipewireReceiver>) {
    info!("[MessageWrapper] Starting Receive Wrapper");
    while let Ok(msg) = recv.recv() {
        if msg == PipewireReceiver::Quit {
            info!("[MessageWrapper] Stopping");
            // Received Quit message, break out.
            break;
        }
        let _ = resend.blocking_send(msg);
    }
    info!("[MessageWrapper] Stopped");
}

pub async fn run_pipewire_manager(config: PipewireManagerConfig, stopped: oneshot::Sender<()>) {
    let mut manager = PipewireManager::new(config);
    manager.run().await;


    drop(manager);
    let _ = stopped.send(());
}

pub(crate) struct PipewireManagerConfig {
    pub(crate) command_receiver: mpsc::Receiver<ManagerMessage>,
    pub(crate) worker_sender: mpsc::Sender<WorkerMessage>,

    pub(crate) ready_sender: Option<oneshot::Sender<()>>,
}