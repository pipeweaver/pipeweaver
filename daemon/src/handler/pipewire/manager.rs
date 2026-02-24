use crate::handler::pipewire::components::application::{
    ApplicationManagement, get_application_type,
};
use crate::handler::pipewire::components::links::LinkManagement;
use crate::handler::pipewire::components::load_profile::LoadProfile;
use crate::handler::pipewire::components::physical::PhysicalDevices;
use crate::handler::pipewire::components::volume::VolumeManager;
use crate::handler::pipewire::ipc::IPCHandler;
use crate::handler::primary_worker::WorkerMessage::TransientChange;
use crate::handler::primary_worker::{ManagerMessage, WorkerMessage};
use crate::servers::http_server::MeterEvent;
use enum_map::{EnumMap, enum_map};
use log::{debug, error, info, warn};
use pipeweaver_ipc::commands::{
    APICommandResponse, Application, AudioConfiguration, PhysicalDevice,
};
use pipeweaver_pipewire::{
    ApplicationNode, DeviceNode, MediaClass, NodeTarget, PipewireMessage, PipewireReceiver,
    PipewireRunner,
};
use pipeweaver_profile::Profile;
use pipeweaver_shared::{AppTarget, DeviceType, Mix};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc::Sender;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use ulid::Ulid;

type StdRecv = std::sync::mpsc::Receiver<PipewireReceiver>;

pub(crate) struct PipewireManager {
    command_receiver: mpsc::Receiver<ManagerMessage>,
    worker_sender: Sender<WorkerMessage>,
    ready_sender: Option<oneshot::Sender<()>>,

    pub(crate) pipewire: Option<PipewireRunner>,
    pub(crate) clock_rate: Option<u32>,
    pub(crate) default_source: Option<NodeTarget>,
    pub(crate) default_target: Option<NodeTarget>,

    pub(crate) profile: Profile,
    pub(crate) source_map: HashMap<Ulid, EnumMap<Mix, Ulid>>,

    // Maps the connection of a PassThrough filter to a Physical Source id
    pub(crate) physical_source: HashMap<Ulid, Vec<u32>>,
    pub(crate) physical_target: HashMap<Ulid, Vec<u32>>,

    // Maps node to a Meter
    pub(crate) meter_enabled: bool,
    pub(crate) meter_map: HashMap<Ulid, Ulid>,
    pub(crate) meter_callback: Sender<(Ulid, u8)>,

    meter_receiver: Option<mpsc::Receiver<(Ulid, u8)>>,
    meter_broadcast: broadcast::Sender<MeterEvent>,

    // A list of physical nodes
    pub(crate) node_list: EnumMap<DeviceType, Vec<PhysicalDevice>>,
    pub(crate) device_nodes: HashMap<u32, DeviceNode>,

    // A list of application nodes
    pub(crate) application_nodes: HashMap<u32, ApplicationNode>,
    pub(crate) application_target_ignore: HashMap<u32, Option<NodeTarget>>,
}

impl PipewireManager {
    pub fn new(config: PipewireManagerConfig) -> Self {
        let (meter_tx, meter_rx) = mpsc::channel(32);

        Self {
            command_receiver: config.command_receiver,
            worker_sender: config.worker_sender,
            ready_sender: config.ready_sender,

            pipewire: None,
            clock_rate: None,
            default_source: None,
            default_target: None,

            profile: config.profile,

            source_map: HashMap::default(),

            physical_source: HashMap::default(),
            physical_target: HashMap::default(),

            meter_enabled: false,
            meter_map: HashMap::default(),
            meter_callback: meter_tx,
            meter_receiver: Some(meter_rx),
            meter_broadcast: config.meter_sender,

            node_list: Default::default(),
            device_nodes: Default::default(),

            application_nodes: Default::default(),
            application_target_ignore: Default::default(),
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
            defaults: enum_map! {
                DeviceType::Source => match &self.default_source {
                    None => None,
                    Some(target) => match target {
                        NodeTarget::Node(id) => Some(AppTarget::Managed(*id)),
                        NodeTarget::UnmanagedNode(id) => Some(AppTarget::Unmanaged(*id)),
                    }
                },
                DeviceType::Target => match &self.default_target {
                    None => None,
                    Some(target) => match target {
                        NodeTarget::Node(id) => Some(AppTarget::Managed(*id)),
                        NodeTarget::UnmanagedNode(id) => Some(AppTarget::Unmanaged(*id)),
                    }
                },
            },
            applications: {
                let mut sources: HashMap<String, HashMap<String, Vec<Application>>> =
                    HashMap::new();
                let mut targets: HashMap<String, HashMap<String, Vec<Application>>> =
                    HashMap::new();

                for (id, application) in &self.application_nodes {
                    let app_type = get_application_type(application.node_class);
                    let map = match app_type {
                        DeviceType::Source => &mut sources,
                        DeviceType::Target => &mut targets,
                    };

                    let app = Application {
                        node_id: *id,
                        name: application.name.clone(),

                        volume: application.volume,
                        muted: application.muted,
                        title: application.title.clone(),

                        target: match application.media_target {
                            None => None,
                            Some(None) => None,
                            Some(Some(target)) => match target {
                                NodeTarget::Node(id) => Some(AppTarget::Managed(id)),
                                NodeTarget::UnmanagedNode(id) => Some(AppTarget::Unmanaged(id)),
                            },
                        },
                    };

                    if let Some(process) = map.get_mut(&application.process_name) {
                        if let Some(title) = process.get_mut(&application.name) {
                            title.push(app);
                        } else {
                            process.insert(application.name.clone(), vec![app]);
                        }
                    } else {
                        map.insert(
                            application.process_name.clone(),
                            HashMap::from([(application.name.clone(), vec![app])]),
                        );
                    }
                }

                enum_map! {
                    DeviceType::Source => sources.clone(),
                    DeviceType::Target => targets.clone(),
                }
            },
        }
    }

    pub async fn run(&mut self) {
        debug!("[Pipewire Runner] Starting Event Loop");
        let (send, recv) = std::sync::mpsc::channel();
        let send_sync = send.clone();

        // We need a largish buffer here because it's impossible to know how much data pipewire is
        // going to throw at us at any given point in time, especially seeing as devices and volume
        // changes are going to flood in, especially on load.
        let (send_async, mut recv_async) = mpsc::channel(2048);
        let send_local_async = send_async.clone();

        // Spawn up the Sync -> Async task loop
        let receiver = thread::spawn(|| run_receiver_wrapper(recv, send_async));

        // Run up the Pipewire Handler
        self.pipewire = Some(PipewireRunner::new(send.clone()).unwrap());

        // Hold until we receive a clock value
        let mut loaded_profile = false;

        // Wait 1 second to process volume inputs from Pipewire
        let mut initial_ready = false;
        let mut initial_ready_timer = Box::pin(sleep(Duration::from_secs(1)));

        // This is a simple timer for applications. Because they won't appear with their default
        // routes, it's best to give them half a second for pipewire to read the data and propagate
        // it. The Pipeweaver frontend will briefly wait before presenting it.
        let mut application_timers: HashMap<u32, JoinHandle<()>> = HashMap::new();
        let (application_ready_tx, mut application_ready_rx) = mpsc::channel(256);

        // A simple list of applications which have been discovered, but not flagged 'Ready'
        let mut discovered_applications: HashMap<u32, ApplicationNode> = HashMap::new();

        // Let the primary worker know we're ready
        let _ = self
            .ready_sender
            .take()
            .expect("Ready Sender Missing")
            .send(());

        let mut requeue: Vec<PipewireReceiver> = vec![];

        // Pull out the Meter Receiver
        let mut meter_receiver = self.meter_receiver.take().unwrap();
        let mut meter_buffer: Vec<(Ulid, u8)> = Vec::with_capacity(64);

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
                        ManagerMessage::SetMetering(enabled) => {
                            let _ = self.set_metering(enabled).await;
                        }
                        ManagerMessage::SetAudioQuantum(value, callback) => {
                            self.profile.audio_quantum = value;
                            let _ = callback.send(());
                        }
                        ManagerMessage::Quit => {
                            info!("[Manager] Stopping");
                            break;
                        }
                    }
                }
                Some(msg) = recv_async.recv() => {
                    if let PipewireReceiver::AnnouncedClock(clock) = msg {
                        if loaded_profile {
                            warn!("Clock Received after Profile Loaded");
                            continue;
                        }

                        if let Some(clock) = clock {
                            self.clock_rate = Some(clock);
                        } else {
                            self.clock_rate = Some(48000);
                        }

                        if !loaded_profile {
                            // Requeue all previous messages before loading the profile, this
                            // needs to be done here to prevent messages during loading from
                            // superseding what we've already received.
                            for msg in &requeue {
                                let _ = send_local_async.send(msg.clone()).await;
                            }

                            debug!("Loading Profile, Clock Rate: {:?}", self.clock_rate);
                            if let Err(e) = self.load_profile().await {
                                error!("Error Loading Profile: {}", e);
                            }
                            loaded_profile = true;
                            continue;
                        }
                    } else if !loaded_profile {
                        // Profile hasn't been loaded yet, queue this for after it has.
                        requeue.push(msg.clone());
                        continue;
                    }

                    match msg {
                        PipewireReceiver::AnnouncedClock(_) => {
                            warn!("This shouldn't happen twice!");
                        }

                        PipewireReceiver::DefaultChanged(class, target) => {
                            match class {
                                MediaClass::Source => {
                                    self.default_source = Some(target);
                                }
                                MediaClass::Sink => {
                                    self.default_target = Some(target);
                                }
                                _ => error!("Invalid MediaClass for Default")
                            }

                            debug!("Default {:?} Changed to {:?}", class, target);
                            let _ = self.worker_sender.send(TransientChange).await;
                        }

                        PipewireReceiver::DeviceAdded(node) => {
                            debug!("Device Found: {:?}, Type: {:?}", node.description, node.node_class);

                            // Create the 'Status' object
                            let physical_node = PhysicalDevice {
                                node_id: node.node_id,
                                name: node.name.clone(),
                                description: node.description.clone(),
                                is_usable: node.is_usable,
                            };

                            let (is_source, is_target) = match node.node_class {
                                MediaClass::Source => (true, false),
                                MediaClass::Sink => (false, true),
                                MediaClass::Duplex => (true, true),
                            };

                            if is_source {
                                self.node_list[DeviceType::Source].push(physical_node.clone());
                                if node.is_usable {
                                    let sender = self.worker_sender.clone();
                                    let _ = self.source_device_added(physical_node.clone(), sender.clone()).await;
                                }
                            }
                            if is_target {
                                self.node_list[DeviceType::Target].push(physical_node.clone());
                                if node.is_usable {
                                    let sender = self.worker_sender.clone();
                                    let _ = self.target_device_added(physical_node, sender).await;
                                }
                            }

                            // Add node to our definitive list
                            self.device_nodes.insert(node.node_id, node);
                            if self.worker_sender.capacity() > 0 {
                                let _ = self.worker_sender.send(TransientChange).await;
                            }
                        }
                        PipewireReceiver::DeviceRemoved(id) => {
                            debug!("Device Removed: {}", id);
                            if let Some(node) = self.device_nodes.remove(&id) {
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
                                let _ = self.worker_sender.send(TransientChange).await;
                            }
                        }
                        PipewireReceiver::DeviceUsable(id, usable) => {
                            if let Some(dev) = self.device_nodes.get_mut(&id) {
                                if dev.is_usable == usable {
                                    continue;
                                }

                                debug!("Usability Changed for Node {}: {} -> {}", id, dev.is_usable, usable);
                                dev.is_usable = usable;

                                let node_class = dev.node_class;

                                // Create physical node for attachment
                                let physical_node = PhysicalDevice {
                                    node_id: id,
                                    name: dev.name.clone(),
                                    description: dev.description.clone(),
                                    is_usable: usable,
                                };

                                // Update node_list
                                match node_class {
                                    MediaClass::Source => {
                                        if let Some(node) = self.node_list[DeviceType::Source]
                                            .iter_mut()
                                            .find(|n| n.node_id == id)
                                        {
                                            node.is_usable = usable;
                                        }
                                    }
                                    MediaClass::Sink => {
                                        if let Some(node) = self.node_list[DeviceType::Target]
                                            .iter_mut()
                                            .find(|n| n.node_id == id)
                                        {
                                            node.is_usable = usable;
                                        }
                                    }
                                    MediaClass::Duplex => {
                                        if let Some(node) = self.node_list[DeviceType::Source]
                                            .iter_mut()
                                            .find(|n| n.node_id == id)
                                        {
                                            node.is_usable = usable;
                                        }
                                        if let Some(node) = self.node_list[DeviceType::Target]
                                            .iter_mut()
                                            .find(|n| n.node_id == id)
                                        {
                                            node.is_usable = usable;
                                        }
                                    }
                                }

                                // Handle connection/disconnection based on usability
                                if usable {
                                    // Device became usable, connect it
                                    match node_class {
                                        MediaClass::Source => {
                                            let sender = self.worker_sender.clone();
                                            let _ = self.source_device_added(physical_node, sender).await;
                                        }
                                        MediaClass::Sink => {
                                            let sender = self.worker_sender.clone();
                                            let _ = self.target_device_added(physical_node, sender).await;
                                        }
                                        MediaClass::Duplex => {
                                            let sender = self.worker_sender.clone();
                                            let _ = self.source_device_added(physical_node.clone(), sender.clone()).await;
                                            let _ = self.target_device_added(physical_node, sender).await;
                                        }
                                    }
                                } else {
                                    // Device became unusable, disconnect it
                                    match node_class {
                                        MediaClass::Source => {
                                            let _ = self.source_device_disconnect(id).await;
                                        }
                                        MediaClass::Sink => {
                                            let _ = self.target_device_disconnect(id).await;
                                        }
                                        MediaClass::Duplex => {
                                            let _ = self.source_device_disconnect(id).await;
                                            let _ = self.target_device_disconnect(id).await;
                                        }
                                    }
                                }
                            }
                            let _ = self.worker_sender.send(TransientChange).await;
                        }
                        PipewireReceiver::ManagedLinkDropped(source, target) => {
                            warn!("Managed Link Removed: {:?} {:?}, reestablishing", source, target);
                            if let Err(e) = self.link_create_type_to_type(source, target).await {
                                warn!("Unable to reestablish link: {}", e);
                            }
                        }
                        PipewireReceiver::ApplicationAdded(node) => {
                            if node.media_target.is_some() {
                                // We already have a target defined, no point waiting for it.
                                let _ = self.application_appeared(node);

                                if self.worker_sender.capacity() > 0 {
                                    let _ = self.worker_sender.send(TransientChange).await;
                                }
                                continue;
                            }

                            if application_timers.contains_key(&node.node_id) {
                                continue;
                            }

                            let done = application_ready_tx.clone();
                            let handle = tokio::spawn(async move {
                                sleep(Duration::from_millis(250)).await;
                                let _ = done.send(node.node_id).await;
                            });
                            application_timers.insert(node.node_id, handle);
                            discovered_applications.insert(node.node_id, node);
                        }
                        PipewireReceiver::ApplicationTargetChanged(id, target) => {
                            if let Some(handle) = application_timers.remove(&id) {
                                if let Some(mut node) = discovered_applications.remove(&id) {
                                    debug!("Route Received During Discovery: {}, {:?}", id, target);

                                    // Ok, set this target, and flag this application as appeared.
                                    node.media_target = Some(target);
                                    let _ = self.application_appeared(node);
                                }
                                handle.abort();
                            } else {
                                let _ = self.application_target_changed(id, Some(target));
                            }

                            if self.worker_sender.capacity() > 0 {
                                let _ = self.worker_sender.send(TransientChange).await;
                            }
                        }
                        PipewireReceiver::ApplicationVolumeChanged(id, volume) => {
                            // Are we still waiting for this application to finalise?
                            if let Some(application) = discovered_applications.get_mut(&id) {
                                debug!("Application Volume Changed during Discovery Phase: {}", volume);
                                application.volume = volume;
                            } else {
                                let _ = self.application_volume_changed(id, volume);
                                if self.worker_sender.capacity() > 0 {
                                    let _ = self.worker_sender.send(TransientChange).await;
                                }
                            }
                        }
                        PipewireReceiver::ApplicationTitleChanged(id, title) => {
                            if let Some(application) = discovered_applications.get_mut(&id) {
                                debug!("Application Title Changed during Discovery Phase: {}", title);
                                application.title = Some(title);
                            } else {
                                let _ = self.application_title_changed(id, title);
                                if self.worker_sender.capacity() > 0 {
                                    let _ = self.worker_sender.send(TransientChange).await;
                                }
                            }
                        }
                        PipewireReceiver::ApplicationMuteChanged(id, muted) => {
                            if let Some(application) = discovered_applications.get_mut(&id) {
                                debug!("Application Muted Changed during Discovery Phase: {}", muted);
                                application.muted = muted;
                            } else {
                                let _ = self.application_mute_changed(id, muted);
                                if self.worker_sender.capacity() > 0 {
                                    let _ = self.worker_sender.send(TransientChange).await;
                                }
                            }
                        }
                        PipewireReceiver::ApplicationRemoved(id) => {
                            // If this application disappears during the 'discovery' phase, clean it
                            if let Some(handle) = application_timers.remove(&id) {
                                debug!("Application Disappeared During Discovery Period: {}", id);
                                discovered_applications.remove(&id);
                                handle.abort();
                            } else {
                                let _ = self.application_removed(id);
                                if self.worker_sender.capacity() > 0 {
                                    let _ = self.worker_sender.send(TransientChange).await;
                                }
                            }
                        }
                        PipewireReceiver::NodeVolumeChanged(id, volume) => {
                            if initial_ready {
                                if let Err(e) = self.sync_node_volume(id, volume).await {
                                    warn!("Error Setting Volume: {}", e);
                                }
                                if self.worker_sender.capacity() > 0 {
                                    let _ = self.worker_sender.send(WorkerMessage::ProfileChanged).await;
                                }
                            }
                        }
                        PipewireReceiver::NodeMuteChanged(id, muted) => {
                            if let Err(e) = self.sync_node_mute(id, muted).await {
                                warn!("Error Setting Mute State: {}", e);
                            }

                            // Save the change in the profile
                            if self.worker_sender.capacity() > 0 {
                                let _ = self.worker_sender.send(WorkerMessage::ProfileChanged).await;
                            }
                        }
                        PipewireReceiver::Quit => {
                            break;
                        }
                    }
                }
                _ = Pin::as_mut(&mut initial_ready_timer), if !initial_ready => {
                    debug!("Activating Pipewire Volume Manager");
                    self.sync_all_pipewire_volumes().await;
                    self.sync_all_pipewire_mutes().await;
                    initial_ready = true;
                }
                Some(node_id) = application_ready_rx.recv() => {
                    // An Application has been hanging around for 200ms without receiving a route,
                    // proceed assuming it's using a default.
                    application_timers.remove(&node_id);
                    if let Some(node) = discovered_applications.remove(&node_id) {
                        debug!("Node Arrived without Route, {}", node_id);
                        let _ = self.application_appeared(node);

                        if self.worker_sender.capacity() > 0 {
                            let _ = self.worker_sender.send(TransientChange).await;
                        }
                    }
                }
                result = meter_receiver.recv_many(&mut meter_buffer, 64) => {
                    if result > 0 {
                        for (id, percent) in meter_buffer.drain(..result) {
                            let _ = self.meter_broadcast.send(MeterEvent {
                                id,
                                percent
                            });
                        }
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
pub fn run_receiver_wrapper(recv: StdRecv, resend: Sender<PipewireReceiver>) {
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
    pub(crate) profile: Profile,

    pub(crate) command_receiver: mpsc::Receiver<ManagerMessage>,
    pub(crate) worker_sender: Sender<WorkerMessage>,

    pub(crate) meter_sender: broadcast::Sender<MeterEvent>,

    pub(crate) ready_sender: Option<oneshot::Sender<()>>,
}
