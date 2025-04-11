use crate::handler::pipewire::components::load_profile::LoadProfile;
use crate::handler::pipewire::ipc::ipc::IPCHandler;
use crate::handler::primary_worker::ManagerMessage;
use enum_map::EnumMap;
use futures::stream::{FuturesUnordered, StreamExt};
use log::{debug, error, warn};
use pipecast_ipc::commands::{AudioConfiguration, PipewireCommandResponse};
use pipecast_pipewire::tokio::sync::oneshot::Receiver;
use pipecast_pipewire::{MediaClass, PipewireNode, PipewireReceiver, PipewireRunner};
use pipecast_profile::{PhysicalDeviceDescriptor, Profile};
use pipecast_shared::Mix;
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};
use tokio::select;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use ulid::Ulid;

type StdRecv = std::sync::mpsc::Receiver<PipewireReceiver>;

#[derive(Copy, Clone)]
pub(crate) struct Waker {
    pub(crate) id: Ulid,
    node_id: u32,
    class: MediaClass,
    created: Instant,
}

pub(crate) struct PipewireManager {
    command_receiver: mpsc::Receiver<ManagerMessage>,

    pub(crate) pipewire: Option<PipewireRunner>,

    pub(crate) profile: Profile,
    pub(crate) source_map: HashMap<Ulid, EnumMap<Mix, Ulid>>,
    pub(crate) target_map: HashMap<Ulid, Ulid>,

    // Maps the connection of a PassThrough filter to a Physical Source id
    pub(crate) physical_source: HashMap<Ulid, Vec<u32>>,
    pub(crate) physical_target: HashMap<Ulid, Vec<u32>>,

    // A 'Waker' is a simple Pipewire Filter that sits and waits for samples to be sent
    // prior to triggering a callback. This prevents any temporary latency caused by a device
    // 'waking up' from being propagated through the entire node tree.
    pub(crate) wake_filters: HashMap<Ulid, Waker>,

    // A list of physical nodes 
    pub(crate) physical_nodes: Vec<PipewireNode>,
}

impl PipewireManager {
    pub fn new(command_receiver: mpsc::Receiver<ManagerMessage>) -> Self {
        Self {
            command_receiver,
            pipewire: None,

            profile: Profile::base_settings(),

            source_map: HashMap::default(),
            target_map: HashMap::default(),

            physical_source: HashMap::default(),
            physical_target: HashMap::default(),

            wake_filters: HashMap::default(),

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
                                self.physical_nodes.retain(|node| node.node_id != id);
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
                        
                        self.physical_nodes.push(device);
                    } else {
                        panic!("Got a Timer Ready for non-existent Node");
                    }
                }
                Some(_) = wakers.next() => {

                }
            );
        }
    }

    //
    // // Keeping this for now :D
    // fn locate_physical_node_id(
    //     &self,
    //     device: &PhysicalDeviceDescriptor,
    //     input: bool,
    // ) -> Option<u32> {
    //     debug!("Looking for Physical Device: {:?}", device);
    //
    //     // This might look a little cumbersome, especially with the need to iterate three
    //     // times, HOWEVER, we have to check this in terms of accuracy. The name is the
    //     // specific location of a device on the USB / PCI-E / etc bus, which if we hit is
    //     // a guaranteed 100% 'this is our device' match.
    //     for node in &self.node_list {
    //         if device.name == node.name
    //             && ((input && node.inputs != 0) || (!input && node.outputs != 0))
    //         {
    //             debug!(
    //                 "Found Name Match {:?}, NodeId: {}",
    //                 device.name, node.node_id
    //             );
    //             return Some(node.node_id);
    //         }
    //     }
    //
    //     // The description is *GENERALLY* unique, and represents how the device is displayed
    //     // in things like pavucontrol, Gnome's and KDE's audio settings, etc., but uniqueness
    //     // is less guaranteed here. This is often useful in situations where (for example)
    //     // the device is plugged into a different USB port, so it's name has changed
    //     if device.description.is_some() {
    //         for node in &self.node_list {
    //             if device.description == node.description
    //                 && ((input && node.inputs != 0) || (!input && node.outputs != 0))
    //             {
    //                 debug!(
    //                     "Found Description Match {:?}, NodeId: {}",
    //                     device.description, node.node_id
    //                 );
    //                 return Some(node.node_id);
    //             }
    //         }
    //     }
    //
    //     // Finally, we'll check by nickname. In my experience this is very NOT unique, for
    //     // example, all my GoXLR nodes have their nickname as 'GoXLR', I'm at least slightly
    //     // skeptical whether I should even track this due to the potential for false
    //     // positives, but it's here for now.
    //     if device.nickname.is_some() {
    //         for node in &self.node_list {
    //             if device.nickname == node.nickname
    //                 && ((input && node.inputs != 0) || (!input && node.outputs != 0))
    //             {
    //                 debug!(
    //                     "Found Nickname Match {:?}, NodeId: {}",
    //                     device.nickname, node.node_id
    //                 );
    //                 return Some(node.node_id);
    //             }
    //         }
    //     }
    //     debug!("Device Not Found: {:?}", device);
    //
    //     None
    // }
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
