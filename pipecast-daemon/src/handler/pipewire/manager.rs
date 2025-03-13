use crate::handler::pipewire::filters::pass_through::PassThroughFilter;
use crate::handler::pipewire::filters::volume::VolumeFilter;
use crate::handler::pipewire::filters::waker::WakerFilter;
use crate::handler::primary_worker::ManagerMessage;
use anyhow::{bail, Result};
use enum_map::{enum_map, EnumMap};
use futures::stream::{FuturesUnordered, StreamExt};
use log::{debug, info};
use pipecast_ipc::commands::{AudioConfiguration, PipewireCommand, PipewireCommandResponse};
use pipecast_pipewire::tokio::sync::oneshot::Receiver;
use pipecast_pipewire::LinkType::{Filter, UnmanagedNode};
use pipecast_pipewire::{FilterProperties, FilterValue, LinkType, MediaClass, NodeProperties, PipecastNode, PipewireMessage, PipewireRunner};
use pipecast_profile::{DeviceDescription, PhysicalDeviceDescriptor, PhysicalSourceDevice, PhysicalTargetDevice, Profile, VirtualSourceDevice, VirtualTargetDevice, Volumes};
use pipecast_shared::Mix;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use ulid::Ulid;

struct Waker {
    id: Ulid,
    node_id: u32,
    class: MediaClass,
    created: Instant,
}

pub(crate) struct PipewireManager {
    command_receiver: mpsc::Receiver<ManagerMessage>,

    pipewire: PipewireRunner,

    profile: Profile,
    source_map: HashMap<Ulid, EnumMap<Mix, Ulid>>,
    target_map: HashMap<Ulid, Ulid>,

    wakers: HashMap<Ulid, Waker>,
    parkers: HashMap<Ulid, Ulid>,

    node_list: Vec<PipecastNode>,
}

impl PipewireManager {
    pub fn new(command_receiver: mpsc::Receiver<ManagerMessage>) -> Self {
        debug!("Establishing Connection to Pipewire..");
        let manager = PipewireRunner::new().unwrap();

        Self {
            command_receiver,
            pipewire: manager,

            profile: Profile::base_settings(),

            source_map: HashMap::default(),
            target_map: HashMap::default(),

            wakers: HashMap::default(),
            parkers: HashMap::default(),

            node_list: Default::default(),
        }
    }

    async fn get_config(&self) -> AudioConfiguration {
        AudioConfiguration {
            profile: self.profile.clone()
        }
    }

    pub async fn run(&mut self) {
        debug!("[Pipewire Runner] Starting Event Loop");

        debug!("[Pipewire Runner] Waiting for Pipewire to Calm..");
        sleep(Duration::from_secs(1)).await;

        let (tx, rx) = oneshot::channel();
        let _ = self.pipewire.send_message(PipewireMessage::GetUsableNodes(tx));
        if let Ok(nodes) = rx.await {
            self.node_list = nodes;
        }

        debug!("Loading Profile");
        let mut wakers = FuturesUnordered::new();
        self.load_profile(&mut wakers).await;

        loop {
            select!(
                Some(command) = self.command_receiver.recv() => {
                    match command {
                        ManagerMessage::Execute(command, tx) => {
                            let result = match command {
                                PipewireCommand::SetVolume(id, mix, volume) => {
                                    self.set_volume(id, mix, volume).await
                                }
                                _ => {
                                    debug!("Received Command: {:?}", command);
                                    Ok(())
                                }
                            };

                            // Map the result to a PW Response and send it
                            let _ = tx.send(match result {
                                Ok(_) => PipewireCommandResponse::Ok,
                                Err(e) => PipewireCommandResponse::Err(e.to_string())
                            });
                        }
                        ManagerMessage::GetConfig(tx) => {
                            debug!("Sending Audio Config");
                            let _ = tx.send(self.get_config().await);
                        }
                    }
                }
                Some(result) = wakers.next() => {
                    if let Ok(id) = result {
                        if let Some(waker) = self.wakers.get(&id) {
                            debug!("[{}] Device Woke up In {:?}, attaching to tree..", id, waker.created.elapsed());

                            let (source, destination) = match waker.class {
                                MediaClass::Source => (UnmanagedNode(waker.node_id), Filter(id)),
                                MediaClass::Sink => (Filter(id), UnmanagedNode(waker.node_id)),
                                MediaClass::Duplex => panic!("Unexpected Duplex!")
                            };

                            // Attach the Original Node to Tree...
                            let (tx, rx) = oneshot::channel();
                            let _ = self.pipewire.send_message(PipewireMessage::CreateDeviceLink(source, destination, tx));
                            let _ = rx.await;

                            // Give it a moment to attach (TODO: Properly!)
                            // sleep(Duration::from_millis(300)).await;

                            // Remove the links for the wake node...
                            let (source, destination) = match waker.class {
                                MediaClass::Source => (UnmanagedNode(waker.node_id), Filter(waker.id)),
                                MediaClass::Sink => (Filter(waker.id), UnmanagedNode(waker.node_id)),
                                MediaClass::Duplex => panic!("Unexpected Duplex!")
                            };
                            let _ = self.pipewire.send_message(PipewireMessage::RemoveDeviceLink(source, destination));
                            let _ = self.pipewire.send_message(PipewireMessage::RemoveFilterNode(waker.id));
                        }
                    }
                }
            );
        }
    }

    async fn set_volume(&mut self, id: Ulid, mix: Mix, volume: u8) -> Result<()> {
        let volume = volume.clamp(0, 100);

        if let Some(source) = self.source_map.get(&id) {
            let node = source[mix];
            let message = PipewireMessage::SetFilterValue(node, 0, FilterValue::UInt8(volume));
            let _ = self.pipewire.send_message(message);


            if let Some(volumes) = self.get_volumes_by_id(id) {
                volumes.volume[mix] = volume;
            }


            return Ok(());
        }

        // Check the Virtual Targets for this node
        if let Some(node) = self.target_map.get(&id) {
            let message = PipewireMessage::SetFilterValue(*node, 0, FilterValue::UInt8(volume));
            let _ = self.pipewire.send_message(message);

            if let Some(config) = self.get_virtual_target_by_id(id) {
                config.volume = volume;
            }

            return Ok(());
        }

        // Finally, check the physical targets
        for device in &self.profile.devices.targets.physical_devices {
            if device.description.id == id {
                let message = PipewireMessage::SetFilterValue(id, 0, FilterValue::UInt8(volume));
                let _ = self.pipewire.send_message(message);

                if let Some(config) = self.get_physical_target_by_id(id) {
                    config.volume = volume;
                }

                return Ok(());
            }
        }
        bail!("Device Not Found")
    }


    async fn load_profile(&mut self, wakers: &mut FuturesUnordered<Receiver<Ulid>>) {
        // Ok, load the physical sources first
        debug!("Creating Physical Source Filters");
        for device in self.profile.devices.sources.physical_devices.clone() {
            self.create_physical_source(&device).await;
        }

        debug!("Creating Virtual Source Nodes");
        for device in self.profile.devices.sources.virtual_devices.clone() {
            self.create_virtual_source(&device).await;
        }

        // Now to do something similar for the target devices..
        debug!("Creating Physical Target Filters");
        for device in self.profile.devices.targets.physical_devices.clone() {
            self.create_physical_target(&device).await;
        }

        debug!("Creating Virtual Source Nodes");
        for device in self.profile.devices.targets.virtual_devices.clone() {
            self.create_virtual_target(&device).await;
        }

        debug!("Applying Routing");
        for (source, targets) in self.profile.routes.clone() {
            for target in targets {
                self.create_device_route(source, target).await;
            }
        }

        // Fetch the Physical Node List (TODO: We need a listener / callback for this)
        debug!("Fetching attached physical nodes");

        // Ok, check the profile physical settings and map the device to the node
        for device in &self.profile.devices.sources.physical_devices {
            for attached_device in &device.attached_devices {
                if let Some(node_id) = self.locate_physical_node_id(attached_device, false) {
                    // Create a 'Wake' filter, and attach this node to it
                    let (filter_id, receiver) = self.create_wake_filter(&device.description, MediaClass::Source).await;
                    debug!("Waiting for NodeId {} to wake..", node_id);
                    self.wakers.insert(device.description.id, Waker {
                        id: filter_id,
                        class: MediaClass::Source,
                        node_id,
                        created: Instant::now(),
                    });
                    wakers.push(receiver);

                    debug!("Attaching {:?} to Wake Node..", attached_device);
                    let (tx, rx) = oneshot::channel();
                    let _ = self.pipewire.send_message(PipewireMessage::CreateDeviceLink(UnmanagedNode(node_id), Filter(filter_id), tx));
                    let _ = rx.await;
                }
            }
        }

        for device in &self.profile.devices.targets.physical_devices {
            for attached_device in &device.attached_devices {
                if let Some(node_id) = self.locate_physical_node_id(attached_device, true) {
                    debug!("Waiting for NodeId {} to Wake..", node_id);
                    // let (filter_id, receiver) = self.create_wake_filter(&device.description, MediaClass::Sink).await;
                    // self.wakers.insert(device.description.id, Waker {
                    //     id: filter_id,
                    //     class: MediaClass::Sink,
                    //     node_id,
                    //     created: Instant::now(),
                    // });
                    // wakers.push(receiver);
                    //
                    // debug!("Attaching {:?} to Wake Node", attached_device);
                    // let (tx, rx) = oneshot::channel();
                    // let _ = self.pipewire.send_message(PipewireMessage::CreateDeviceLink(Filter(filter_id), UnmanagedNode(node_id), tx));
                    // let _ = rx.await;

                    debug!("Attaching {:?} to {:?}", attached_device, device.description.name);
                    let (tx, rx) = oneshot::channel();
                    let _ = self.pipewire.send_message(PipewireMessage::CreateDeviceLink(Filter(device.description.id), UnmanagedNode(node_id), tx));
                }
            }
        }
    }

    async fn create_physical_source(&mut self, device: &PhysicalSourceDevice) {
        debug!("[{}] Creating Physical Node {}", device.description.id, device.description.name);
        //self.create_node(device.description.clone(), MediaClass::Source).await;
        self.create_pass_through_filter(device.description.clone()).await;

        debug!("[{}] Creating Volume Filters", device.description.id);
        // Create the A and B volume nodes (there might be a nicer way to do this)
        let id_a = Ulid::new();
        let filter_description = DeviceDescription {
            id: id_a,
            name: format!("{} A", device.description.name),
            colour: Default::default(),
        };
        self.create_volume_filter(filter_description, device.volumes.volume[Mix::A]).await;

        let id_b = Ulid::new();
        let filter_description = DeviceDescription {
            id: id_b,
            name: format!("{} B", device.description.name),
            colour: Default::default(),
        };
        self.create_volume_filter(filter_description, device.volumes.volume[Mix::B]).await;

        // Store these Mix Node IDs
        self.source_map.insert(device.description.id, enum_map! {
                Mix::A => id_a,
                Mix::B => id_b
            });

        // Route the filter to the volumes...
        self.create_route(LinkType::Filter(device.description.id), LinkType::Filter(id_a)).await;
        self.create_route(LinkType::Filter(device.description.id), LinkType::Filter(id_b)).await;
    }

    async fn create_virtual_source(&mut self, device: &VirtualSourceDevice) {
        debug!("[{}] Creating Virtual Node {}", device.description.id, device.description.name);
        self.create_node(device.description.clone(), MediaClass::Sink).await;

        debug!("[{}] Creating Volume Filters", device.description.id);
        // Create the A and B volume nodes (there might be a nicer way to do this)
        let id_a = Ulid::new();
        let filter_description = DeviceDescription {
            id: id_a,
            name: format!("{} A", device.description.name),
            colour: Default::default(),
        };
        self.create_volume_filter(filter_description, device.volumes.volume[Mix::A]).await;

        let id_b = Ulid::new();
        let filter_description = DeviceDescription {
            id: id_b,
            name: format!("{} B", device.description.name),
            colour: Default::default(),
        };
        self.create_volume_filter(filter_description, device.volumes.volume[Mix::B]).await;

        // Store these Mix Node IDs
        self.source_map.insert(device.description.id, enum_map! {
                Mix::A => id_a,
                Mix::B => id_b
            });

        // Route the Node to the Volume Filters
        self.create_route(LinkType::Node(device.description.id), LinkType::Filter(id_a)).await;
        self.create_route(LinkType::Node(device.description.id), LinkType::Filter(id_b)).await;
    }

    async fn create_physical_target(&mut self, device: &PhysicalTargetDevice) {
        debug!("[{}] Creating Physical Filter {}", device.description.id, device.description.name);
        self.create_volume_filter(device.description.clone(), device.volume).await;

        // TODO: Attach physical devices
    }

    async fn create_virtual_target(&mut self, device: &VirtualTargetDevice) {
        debug!("[{}] Creating Virtual Node {}", device.description.id, device.description.name);
        self.create_node(device.description.clone(), MediaClass::Source).await;

        debug!("[{}] Creating Volume Filter", device.description.id);
        // Create the A and B volume nodes (there might be a nicer way to do this)
        let id = Ulid::new();
        let filter_description = DeviceDescription {
            id,
            name: device.description.name.to_string(),
            colour: Default::default(),
        };
        self.create_volume_filter(filter_description, device.volume).await;

        // Route the Volume Filter to the Virtual Node
        self.create_route(LinkType::Filter(id), LinkType::Node(device.description.id)).await;
        self.target_map.insert(device.description.id, id);
    }

    fn locate_physical_node_id(&self, device: &PhysicalDeviceDescriptor, input: bool) -> Option<u32> {
        debug!("Looking for Physical Device: {:?}", device);

        // This might look a little cumbersome, especially with the need to iterate three
        // times, HOWEVER, we have to check this in terms of accuracy. The name is the
        // specific location of a device on the USB / PCI-E / etc bus, which if we hit is
        // a guaranteed 100% 'this is our device' match.
        for node in &self.node_list {
            if device.name == node.name && ((input && node.inputs != 0) || (!input && node.outputs != 0)) {
                debug!("Found Name Match {:?}, NodeId: {}", device.name, node.node_id);
                return Some(node.node_id);
            }
        }

        // The description is *GENERALLY* unique, and represents how the device is displayed
        // in things like pavucontrol, Gnome's and KDE's audio settings, etc., but uniqueness
        // is less guaranteed here. This is often useful in situations where (for example)
        // the device is plugged into a different USB port, so it's name has changed
        if device.description.is_some() {
            for node in &self.node_list {
                if device.description == node.description && ((input && node.inputs != 0) || (!input && node.outputs != 0)) {
                    debug!("Found Description Match {:?}, NodeId: {}", device.description, node.node_id);
                    return Some(node.node_id);
                }
            }
        }

        // Finally, we'll check by nickname. In my experience this is very NOT unique, for
        // example, all my GoXLR nodes have their nickname as 'GoXLR', I'm at least slightly
        // skeptical whether I should even track this due to the potential for false
        // positives, but it's here for now.
        if device.nickname.is_some() {
            for node in &self.node_list {
                if device.nickname == node.nickname && ((input && node.inputs != 0) || (!input && node.outputs != 0)) {
                    debug!("Found Nickname Match {:?}, NodeId: {}", device.nickname, node.node_id);
                    return Some(node.node_id);
                }
            }
        }
        debug!("Device Not Found: {:?}", device);

        None
    }

    async fn create_node(&mut self, device: DeviceDescription, class: MediaClass) {
        // Ok, we've been asked to create a node, so let's do that
        let (send, recv) = oneshot::channel();
        let identifier = format!("PipeCast {}", device.name).to_lowercase().replace(" ", "_");
        let props = NodeProperties {
            node_id: device.id,
            node_name: identifier.clone(),
            node_nick: identifier,
            node_description: format!("PipeCast {}", device.name),
            app_id: "com.github.pipecast".to_string(),
            app_name: "pipecast".to_string(),
            linger: false,
            class,
            ready_sender: send,
        };

        let _ = self.pipewire.send_message(PipewireMessage::CreateDeviceNode(props));
        let _ = recv.await;
    }

    async fn remove_node(&mut self) {}

    async fn create_volume_filter(&mut self, device: DeviceDescription, volume: u8) {
        let (send, recv) = oneshot::channel();

        let description = device.name.to_lowercase().replace(" ", "-");
        let props = FilterProperties {
            filter_id: device.id,
            filter_name: "Volume".into(),
            filter_nick: device.name.to_string(),
            filter_description: format!("pipecast/{}", description),

            class: MediaClass::Duplex,
            app_id: "com.frostycoolslug".to_string(),
            app_name: "pipecast".to_string(),
            linger: false,
            callback: Box::new(VolumeFilter::new(volume)),
            ready_sender: send,
        };

        let _ = self.pipewire.send_message(PipewireMessage::CreateFilterNode(props));
        let _ = recv.await;
    }

    async fn create_pass_through_filter(&mut self, device: DeviceDescription) {
        let (send, recv) = oneshot::channel();

        let description = device.name.to_lowercase().replace(" ", "-");
        let props = FilterProperties {
            filter_id: device.id,
            filter_name: "Pass".into(),
            filter_nick: device.name.to_string(),
            filter_description: format!("pipecast/{}", description),

            class: MediaClass::Duplex,
            app_id: "com.frostycoolslug".to_string(),
            app_name: "pipecast".to_string(),
            linger: false,
            callback: Box::new(PassThroughFilter::new()),
            ready_sender: send,
        };

        let _ = self.pipewire.send_message(PipewireMessage::CreateFilterNode(props));
        let _ = recv.await;
    }

    async fn create_wake_filter(&self, device: &DeviceDescription, class: MediaClass) -> (Ulid, Receiver<Ulid>) {
        let (send, recv) = oneshot::channel();
        let (wake_tx, wake_rx) = oneshot::channel();

        let filter_id = Ulid::new();

        let description = device.name.to_lowercase().replace(" ", "-");
        let props = FilterProperties {
            filter_id,
            filter_name: "Wake".into(),
            filter_nick: device.name.to_string(),
            filter_description: format!("pipecast/{}", description),

            class,
            app_id: "com.frostycoolslug".to_string(),
            app_name: "pipecast".to_string(),
            linger: false,
            callback: Box::new(WakerFilter::new(device.id, wake_tx, class)),
            ready_sender: send,
        };

        let _ = self.pipewire.send_message(PipewireMessage::CreateFilterNode(props));
        let _ = recv.await;

        (filter_id, wake_rx)
    }

    async fn remove_filter(&mut self, id: Ulid) {}

    async fn create_device_route(&mut self, source: Ulid, target: Ulid) {
        // This is a little convoluted, as we need to determine what Mix the target device is
        // attached to, as well as the target type (Filter or Node), so a little more iteration
        // here than I would like. I might do some caching or stuff later.

        // Firstly, check the Physical devices
        for device in self.profile.devices.targets.physical_devices.clone() {
            if device.description.id == target {
                // Found it, link our source Mix to match the target..
                if let Some(source) = self.source_map.get(&source) {
                    debug!("[{}][{}] Adding Route", source[device.mix], device.description.id);

                    self.create_route(
                        LinkType::Filter(source[device.mix]),
                        LinkType::Filter(device.description.id),
                    ).await;
                    return;
                }
            }
        }

        // Now, check the Virtual Devices
        for device in self.profile.devices.targets.virtual_devices.clone() {
            if device.description.id == target {
                // Found it, link our source Mix to match the target..
                if let Some(source) = self.source_map.get(&source) {
                    if let Some(target) = self.target_map.get(&device.description.id) {
                        debug!("[{}][{}] Adding Route", source[device.mix], device.description.id);
                        self.create_route(
                            LinkType::Filter(source[device.mix]),
                            LinkType::Filter(*target),
                        ).await;
                    }
                    return;
                }
            }
        }
    }

    async fn create_route(&mut self, source: LinkType, target: LinkType) {
        // Relatively simple, just send a new route message...
        let (tx, rx) = oneshot::channel();
        let _ = self.pipewire.send_message(PipewireMessage::CreateDeviceLink(source, target, tx));
        let _ = rx.await;
    }

    async fn remove_route(&mut self, source: Ulid, destination: Ulid) {}

    fn get_volumes_by_id(&mut self, id: Ulid) -> Option<&mut Volumes> {
        // Determine which device contains the volumes first, without returning references
        let device_id = if self.get_physical_source_by_id(id).is_some() {
            DeviceType::Physical
        } else if self.get_virtual_source_by_id(id).is_some() {
            DeviceType::Virtual
        } else {
            DeviceType::None
        };

        // Retrieve the volumes based on the resolved device type
        match device_id {
            DeviceType::Physical => self.get_physical_source_by_id(id).map(|device| &mut device.volumes),
            DeviceType::Virtual => self.get_virtual_source_by_id(id).map(|device| &mut device.volumes),
            DeviceType::None => None,
        }
    }


    /**
     * These are basically helper functions for finding specific devices by ID as they may
     * occur in different places, they're used primarily for allowing easy grabbing of 'common'
     * structures.
     **/
    fn get_physical_source_by_id(&mut self, id: Ulid) -> Option<&mut PhysicalSourceDevice> {
        self.profile.devices.sources.physical_devices.iter_mut().find(|device| device.description.id == id)
    }

    fn get_virtual_source_by_id(&mut self, id: Ulid) -> Option<&mut VirtualSourceDevice> {
        self.profile.devices.sources.virtual_devices.iter_mut().find(|device| device.description.id == id)
    }

    fn get_physical_target_by_id(&mut self, id: Ulid) -> Option<&mut PhysicalTargetDevice> {
        self.profile.devices.targets.physical_devices.iter_mut().find(|device| device.description.id == id)
    }

    fn get_virtual_target_by_id(&mut self, id: Ulid) -> Option<&mut VirtualTargetDevice> {
        self.profile.devices.targets.virtual_devices.iter_mut().find(|device| device.description.id == id)
    }
}

enum DeviceType {
    Physical,
    Virtual,
    None,
}

pub async fn run_pipewire_manager(command_receiver: mpsc::Receiver<ManagerMessage>) {
    let mut manager = PipewireManager::new(command_receiver);
    manager.run().await;
}