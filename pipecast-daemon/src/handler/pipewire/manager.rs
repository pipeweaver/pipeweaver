use crate::handler::pipewire::filters::pass_through::PassThroughFilter;
use crate::handler::pipewire::filters::volume::VolumeFilter;
use enum_map::{enum_map, EnumMap};
use log::{debug, info};
use pipecast_pipewire::{oneshot, FilterProperties, LinkType, MediaClass, NodeProperties, PipewireMessage, PipewireRunner};
use pipecast_profile::{DeviceDescription, Mix, PhysicalSourceDevice, PhysicalTargetDevice, Profile, VirtualSourceDevice, VirtualTargetDevice};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use ulid::Ulid;

pub(crate) struct PipewireManager {
    pipewire: PipewireRunner,

    profile: Profile,
    source_map: HashMap<Ulid, EnumMap<Mix, Ulid>>,
    target_map: HashMap<Ulid, Ulid>,
}

impl PipewireManager {
    pub fn new() -> Self {
        debug!("Establishing Connection to Pipewire..");
        let manager = PipewireRunner::new().unwrap();

        Self {
            pipewire: manager,
            profile: Default::default(),

            source_map: HashMap::default(),
            target_map: HashMap::default(),
        }
    }

    pub async fn run(&mut self) {
        debug!("[Pipewire Runner] Starting Event Loop");

        debug!("Loading Profile");
        self.load_profile().await;

        loop {
            // We're just going to loop for now..
            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn load_profile(&mut self) {
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
        };
        self.create_volume_filter(filter_description, device.volumes[Mix::A]).await;

        let id_b = Ulid::new();
        let filter_description = DeviceDescription {
            id: id_b,
            name: format!("{} B", device.description.name),
        };
        self.create_volume_filter(filter_description, device.volumes[Mix::B]).await;

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
        };
        self.create_volume_filter(filter_description, device.volumes[Mix::A]).await;

        let id_b = Ulid::new();
        let filter_description = DeviceDescription {
            id: id_b,
            name: format!("{} B", device.description.name),
        };
        self.create_volume_filter(filter_description, device.volumes[Mix::B]).await;

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
        };
        self.create_volume_filter(filter_description, device.volume).await;

        // Route the Volume Filter to the Virtual Node
        self.create_route(LinkType::Filter(id), LinkType::Node(device.description.id)).await;
        self.target_map.insert(device.description.id, id);
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
            app_id: "com.frostycoolslug".to_string(),
            app_name: "pipecast".to_string(),
            linger: false,
            callback: Box::new(PassThroughFilter::new()),
            ready_sender: send,
        };

        let _ = self.pipewire.send_message(PipewireMessage::CreateFilterNode(props));
        let _ = recv.await;
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
        let _ = self.pipewire.send_message(PipewireMessage::CreateDeviceLink(source, target));
    }

    async fn remove_route(&mut self, source: Ulid, destination: Ulid) {}
}

pub async fn run_pipewire_manager() {
    let mut manager = PipewireManager::new();
    manager.run().await;
}