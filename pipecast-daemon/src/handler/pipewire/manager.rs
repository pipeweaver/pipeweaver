use crate::handler::pipewire::components::load_profile::LoadProfile;
use crate::handler::pipewire::components::physical::PhysicalDevices;
use crate::handler::pipewire::ipc::ipc::IPCHandler;
use crate::handler::primary_worker::ManagerMessage;
use enum_map::EnumMap;
use futures::stream::{FuturesUnordered, StreamExt};
use log::{debug, error, warn};
use pipecast_ipc::commands::{AudioConfiguration, PhysicalDevice, PipewireCommandResponse};
use pipecast_pipewire::tokio::sync::oneshot::Receiver;
use pipecast_pipewire::{MediaClass, PipewireNode, PipewireReceiver, PipewireRunner};
use pipecast_profile::{DeviceDescription, PhysicalDeviceDescriptor, Profile};
use pipecast_shared::{DeviceType, Mix};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use ulid::Ulid;

type StdRecv = std::sync::mpsc::Receiver<PipewireReceiver>;

pub(crate) struct PipewireManager {
    command_receiver: mpsc::Receiver<ManagerMessage>,

    pub(crate) pipewire: Option<PipewireRunner>,

    pub(crate) profile: Profile,
    pub(crate) source_map: HashMap<Ulid, EnumMap<Mix, Ulid>>,
    pub(crate) target_map: HashMap<Ulid, Ulid>,

    // Maps the connection of a PassThrough filter to a Physical Source id
    pub(crate) physical_source: HashMap<Ulid, Vec<u32>>,
    pub(crate) physical_target: HashMap<Ulid, Vec<u32>>,

    // A list of physical nodes
    pub(crate) node_list: EnumMap<DeviceType, Vec<PhysicalDevice>>,
    pub(crate) physical_nodes: Vec<PipewireNode>,
}

impl PipewireManager {
    pub fn new(command_receiver: mpsc::Receiver<ManagerMessage>) -> Self {
        Self {
            command_receiver,
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

    async fn get_config(&self) -> AudioConfiguration {
        AudioConfiguration {
            profile: self.profile.clone(),
            devices: self.node_list.clone(),
        }
    }

    pub async fn run(&mut self) {
        debug!("[Pipewire Runner] Starting Event Loop");
        let (send, recv) = std::sync::mpsc::channel();
        let (send_async, mut recv_async) = mpsc::channel(1024);

        // Spawn up the Sync -> Async task loop
        thread::spawn(|| run_receiver_wrapper(recv, send_async));

        // Run up the Pipewire Handler
        self.pipewire = Some(PipewireRunner::new(send.clone()).unwrap());


        debug!("[Pipewire Runner] Waiting for Pipewire to Calm..");
        sleep(Duration::from_secs(1)).await;

        // let (tx, rx) = oneshot::channel();
        // let _ = self.pipewire().send_message(PipewireMessage::GetUsableNodes(tx));
        // if let Ok(nodes) = rx.await {
        //     self.node_list = nodes;
        // }

        debug!("Loading Profile");
        if let Err(e) = self.load_profile().await {
            error!("Error Loading Profile: {}", e);
        }

        // Callback Handlers for the Wakers
        let mut wakers: FuturesUnordered<Receiver<Ulid>> = FuturesUnordered::new();

        // So these are small timers which are set up when a new device is sent to us, rather than
        // immediately processing we wait a second to make sure the device doesn't immediately
        // disappear again as it's layout is being calculated, if this timer completes we should
        // be happy to assume the device configuration has stabilised.
        let mut device_timers: HashMap<u32, JoinHandle<()>> = HashMap::new();
        let mut discovered_devices: HashMap<u32, PipewireNode> = HashMap::new();


        let (device_ready_tx, mut device_ready_rx) = mpsc::channel(1024);


        loop {
            select!(
                Some(command) = self.command_receiver.recv() => {
                    match command {
                        ManagerMessage::Execute(command, tx) => {
                            let result = self.handle_command(command).await;

                            // Map the result to a PW Response and send it
                            let _ = tx.send(match result {
                                Ok(response) => response,
                                Err(e) => PipewireCommandResponse::Err(e.to_string())
                            });
                        }
                        ManagerMessage::GetConfig(tx) => {
                            let _ = tx.send(self.get_config().await);
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
                                sleep(Duration::from_secs(1)).await;
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
                                let node = self.physical_nodes.iter().find(|node| node.node_id == id);
                                if let Some(node) = node {
                                    // We also need to remove this from the IPC list
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
                                    self.physical_nodes.retain(|node| node.node_id != id);
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
                    // A device has been sat here for a second without being removed
                    
                    if let Some(device) = discovered_devices.remove(&node_id) {

                        debug!("Device Found: {:?}, Type: {:?}", device.description, device.node_class);
                        device_timers.remove(&node_id);

                        // Create the 'Status' object
                        let node = PhysicalDevice {
                            id: device.node_id,
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
                        self.physical_nodes.push(device);
                    } else {
                        panic!("Got a Timer Ready for non-existent Node");
                    }
                }
            );
        }
    }
}

// Kinda ugly, but we're going to wrap around a blocking receiver, and bounce messages to an async
pub fn run_receiver_wrapper(recv: StdRecv, resend: mpsc::Sender<PipewireReceiver>) {
    while let Ok(msg) = recv.recv() {
        if msg == PipewireReceiver::Quit {
            // Received Quit message, break out.
            break;
        }
        let _ = resend.blocking_send(msg);
    }
}

pub async fn run_pipewire_manager(command_receiver: mpsc::Receiver<ManagerMessage>) {
    let mut manager = PipewireManager::new(command_receiver);
    manager.run().await;
}
