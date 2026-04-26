use crate::handler::pipewire::components::filters::FilterManagement;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::routing::RoutingManagement;
use crate::handler::pipewire::components::volume::VolumeManager;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::Result;
use log::debug;
use pipeweaver_profile::DeviceDescription;
use pipeweaver_shared::{NodeType, OrderGroup};
use ulid::Ulid;

pub const MAX_NODE_NAME_LENGTH: usize = 20;

pub(crate) trait LoadProfile {
    async fn load_profile(&mut self) -> Result<()>;
    fn get_node_id_by_name(&self, name: &str) -> Option<Ulid>;
    fn is_valid_name(name: &str) -> bool;
}

impl LoadProfile for PipewireManager {
    async fn load_profile(&mut self) -> Result<()> {
        self.profile_create_nodes().await?;
        self.profile_load_volumes().await?;
        self.profile_apply_routing().await?;

        #[cfg(feature = "lv2")]
        {
            use crate::handler::pipewire::components::audio_filters::lv2::filters::generic::filter_get_generic_lv2_props;
            use crate::handler::pipewire::components::filters::FilterManagement;
            use pipeweaver_pipewire::FilterValue;
            use pipeweaver_pipewire::{PipewireMessage, oneshot};
            use std::collections::HashMap;
            use ulid::Ulid;

            let uri = "http://lsp-plug.in/plugins/lv2/comp_delay_x2_stereo";
            let name = String::from("Generic LV2 Filter");

            let mut defaults = HashMap::new();
            defaults.insert("enabled".into(), FilterValue::Bool(true));
            defaults.insert("mode_l".into(), FilterValue::Int32(2));
            defaults.insert("mode_r".into(), FilterValue::Int32(2));
            defaults.insert("time_l".into(), FilterValue::Float32(1000.));
            defaults.insert("time_r".into(), FilterValue::Float32(1000.));

            let props = filter_get_generic_lv2_props(uri, name, Ulid::new(), defaults);
            let id = props.filter_id;

            //let props = filter_get_delay_props(String::from("Test"), Ulid::new());
            self.filter_debug_create(props).await?;

            // Ok, lets try and fetch the properties for this filter...
            let (tx, rx) = oneshot::channel();
            let message = PipewireMessage::GetFilterParameters(id, tx);
            self.pipewire().send_message(message)?;
            debug!("{:?}", rx.recv()?);
        }

        Ok(())
    }

    fn get_node_id_by_name(&self, name: &str) -> Option<Ulid> {
        for device in &self.profile.devices.sources.physical_devices {
            if device.description.name == name {
                return Some(device.description.id);
            }
        }

        for device in &self.profile.devices.sources.virtual_devices {
            if device.description.name == name {
                return Some(device.description.id);
            }
        }

        for device in &self.profile.devices.targets.physical_devices {
            if device.description.name == name {
                return Some(device.description.id);
            }
        }

        for device in &self.profile.devices.targets.virtual_devices {
            if device.description.name == name {
                return Some(device.description.id);
            }
        }

        // This name wasn't found, so return none
        None
    }

    fn is_valid_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= MAX_NODE_NAME_LENGTH
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == ' ' || c == '_' || c == '-')
    }
}

trait LoadProfileLocal {
    async fn profile_create_nodes(&mut self) -> Result<()>;
    async fn profile_load_volumes(&mut self) -> Result<()>;
    async fn profile_apply_routing(&mut self) -> Result<()>;
    fn check_device_order_present(&mut self, dev: &DeviceDescription, source: bool) -> Result<()>;
    fn validate_name(description: &mut DeviceDescription, all_devices: &mut Vec<(Ulid, String)>);
    fn validate_device_order(&mut self, source: bool) -> Result<()>;
}

impl LoadProfileLocal for PipewireManager {
    async fn profile_create_nodes(&mut self) -> Result<()> {
        // Collect all device (id, name) pairs for uniqueness checking
        let mut all_devices: Vec<(Ulid, String)> = Vec::new();
        for device in &self.profile.devices.sources.physical_devices {
            all_devices.push((device.description.id, device.description.name.clone()));
        }
        for device in &self.profile.devices.sources.virtual_devices {
            all_devices.push((device.description.id, device.description.name.clone()));
        }
        for device in &self.profile.devices.targets.physical_devices {
            all_devices.push((device.description.id, device.description.name.clone()));
        }
        for device in &self.profile.devices.targets.virtual_devices {
            all_devices.push((device.description.id, device.description.name.clone()));
        }

        // Validate names for all devices
        for device in &mut self.profile.devices.sources.physical_devices {
            Self::validate_name(&mut device.description, &mut all_devices);
        }
        for device in &mut self.profile.devices.sources.virtual_devices {
            Self::validate_name(&mut device.description, &mut all_devices);
        }
        for device in &mut self.profile.devices.targets.physical_devices {
            Self::validate_name(&mut device.description, &mut all_devices);
        }
        for device in &mut self.profile.devices.targets.virtual_devices {
            Self::validate_name(&mut device.description, &mut all_devices);
        }

        // Second pass: create nodes and check device order
        let mut physical_sources = Vec::new();
        for device in &self.profile.devices.sources.physical_devices {
            physical_sources.push(device.description.clone());
        }
        for desc in &physical_sources {
            self.node_create(NodeType::PhysicalSource, desc).await?;
            self.node_load_filters(desc.id).await?;
            self.check_device_order_present(desc, true)?;
        }

        let mut virtual_sources = Vec::new();
        for device in &self.profile.devices.sources.virtual_devices {
            virtual_sources.push(device.description.clone());
        }
        for desc in &virtual_sources {
            self.node_create(NodeType::VirtualSource, desc).await?;
            self.node_load_filters(desc.id).await?;
            self.check_device_order_present(desc, true)?;
        }
        self.validate_device_order(true)?;

        let mut physical_targets = Vec::new();
        for device in &self.profile.devices.targets.physical_devices {
            physical_targets.push(device.description.clone());
        }
        for desc in &physical_targets {
            self.node_create(NodeType::PhysicalTarget, desc).await?;
            self.node_load_filters(desc.id).await?;
            self.check_device_order_present(desc, false)?;
        }

        let mut virtual_targets = Vec::new();
        for device in &self.profile.devices.targets.virtual_devices {
            virtual_targets.push(device.description.clone());
        }
        for desc in &virtual_targets {
            self.node_create(NodeType::VirtualTarget, desc).await?;
            self.node_load_filters(desc.id).await?;
            self.check_device_order_present(desc, false)?;
        }
        self.validate_device_order(false)?;

        Ok(())
    }

    async fn profile_load_volumes(&mut self) -> Result<()> {
        self.volumes_load().await
    }

    async fn profile_apply_routing(&mut self) -> Result<()> {
        self.routing_load().await
    }

    fn check_device_order_present(&mut self, dev: &DeviceDescription, source: bool) -> Result<()> {
        let order_list = if source {
            &mut self.profile.devices.sources.device_order
        } else {
            &mut self.profile.devices.targets.device_order
        };

        if !order_list.iter().any(|(_, v)| v.contains(&dev.id)) {
            debug!("Device Not Found in Order List, adding to Default List");
            order_list[OrderGroup::default()].push(dev.id);
        }

        Ok(())
    }

    fn validate_name(description: &mut DeviceDescription, all_devices: &mut Vec<(Ulid, String)>) {
        // Before we do anything, check whether this name is valid, and if not, immediately
        // sanitise it
        if !Self::is_valid_name(&description.name) {
            let sanitized: String = description
                .name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_' || *c == '-')
                .take(MAX_NODE_NAME_LENGTH)
                .collect();

            description.name = if sanitized.is_empty() {
                "UNNAMED".to_string()
            } else {
                sanitized
            };
        }

        let id = description.id;
        let base_name = description.name.clone();
        let mut count = 0;
        loop {
            let name = &description.name;
            let mut found_duplicate = false;
            for (other_id, other_name) in all_devices.iter() {
                if *other_id != id && other_name == name {
                    found_duplicate = true;
                    break;
                }
            }
            if found_duplicate {
                count += 1;
                description.name = format!("{}_{}", base_name, count);
                // Update the name in all_devices for this id
                for (other_id, other_name) in all_devices.iter_mut() {
                    if *other_id == id {
                        *other_name = description.name.clone();
                    }
                }
            } else {
                // Update the name in all_devices for this id (in case it changed)
                for (other_id, other_name) in all_devices.iter_mut() {
                    if *other_id == id {
                        *other_name = description.name.clone();
                    }
                }
                break;
            }
        }
        // When we get here, we should be unique.
    }

    fn validate_device_order(&mut self, source: bool) -> Result<()> {
        // We're looking for devices which may be present in the order, but don't exist
        let mut known_ids = vec![];
        let device_order = if source {
            for device in &self.profile.devices.sources.physical_devices {
                known_ids.push(device.description.id);
            }
            for device in &self.profile.devices.sources.virtual_devices {
                known_ids.push(device.description.id);
            }
            &mut self.profile.devices.sources.device_order
        } else {
            for device in &self.profile.devices.targets.physical_devices {
                known_ids.push(device.description.id);
            }
            for device in &self.profile.devices.targets.virtual_devices {
                known_ids.push(device.description.id);
            }
            &mut self.profile.devices.targets.device_order
        };

        // We'll use a .retain on each element of the order to clean up
        for vec in device_order.values_mut() {
            vec.retain(|id| known_ids.contains(id));
        }
        Ok(())
    }
}
