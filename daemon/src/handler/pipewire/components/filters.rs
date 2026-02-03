use crate::handler::pipewire::components::audio_filters::internal::meter::MeterFilter;
use crate::handler::pipewire::components::audio_filters::internal::pass_through::PassThroughFilter;
use crate::handler::pipewire::components::audio_filters::internal::volume::VolumeFilter;
use crate::handler::pipewire::components::audio_filters::lv2::filters::generic::filter_lv2;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::manager::PipewireManager;
use crate::{APP_ID, APP_NAME, APP_NAME_ID};
use anyhow::{Result, anyhow, bail};
use pipeweaver_pipewire::oneshot;
use pipeweaver_pipewire::{FilterProperties, MediaClass, PipewireMessage};
use pipeweaver_profile::{Filter, FilterConfig};
use pipeweaver_shared::{FilterValue, NodeType};
use std::collections::HashMap;
use ulid::Ulid;

#[allow(unused)]
pub(crate) trait FilterManagement {
    async fn filter_pass_create(&mut self, name: String) -> Result<Ulid>;
    async fn filter_pass_create_id(&mut self, name: String, id: Ulid) -> Result<()>;

    async fn filter_volume_create(&mut self, name: String) -> Result<Ulid>;
    async fn filter_volume_create_id(&mut self, name: String, id: Ulid) -> Result<()>;

    async fn filter_meter_create(&mut self, node: Ulid, name: String) -> Result<Ulid>;
    async fn filter_meter_create_id(&mut self, node: Ulid, name: String, id: Ulid) -> Result<()>;

    async fn filter_volume_set(&self, id: Ulid, volume: u8) -> Result<()>;

    async fn filter_remove(&mut self, id: Ulid) -> Result<()>;
    async fn filter_debug_create(&mut self, props: FilterProperties) -> Result<()>;

    async fn node_load_filters(&mut self, id: Ulid) -> Result<()>;
    async fn filter_custom_create(&mut self, target: Ulid, filter: Filter) -> Result<()>;
}

impl FilterManagement for PipewireManager {
    async fn filter_pass_create(&mut self, name: String) -> Result<Ulid> {
        let id = Ulid::new();
        self.filter_pass_create_id(name, id).await?;

        Ok(id)
    }
    async fn filter_pass_create_id(&mut self, name: String, id: Ulid) -> Result<()> {
        let props = self.filter_pass_get_props(name, id);
        self.filter_pw_create(props).await
    }

    async fn filter_volume_create(&mut self, name: String) -> Result<Ulid> {
        let id = Ulid::new();
        self.filter_volume_create_id(name, id).await?;

        Ok(id)
    }
    async fn filter_volume_create_id(&mut self, name: String, id: Ulid) -> Result<()> {
        let props = self.filter_volume_get_props(name, id);
        self.filter_pw_create(props).await
    }

    async fn filter_meter_create(&mut self, node: Ulid, name: String) -> Result<Ulid> {
        let id = Ulid::new();
        self.filter_meter_create_id(node, name, id).await?;

        Ok(id)
    }

    async fn filter_meter_create_id(&mut self, node: Ulid, name: String, id: Ulid) -> Result<()> {
        let props = self.filter_meter_get_props(node, name, id);
        self.filter_pw_create(props).await
    }

    async fn filter_volume_set(&self, id: Ulid, volume: u8) -> Result<()> {
        if !(0..=100).contains(&volume) {
            bail!("Volume must be between 0 and 100");
        }

        // Establish the custom channel
        let (tx, rx) = oneshot::channel();

        // Define the Value
        let value = FilterValue::UInt8(volume);

        // Send the Message
        let message = PipewireMessage::SetFilterValue(id, 0, value, tx);
        let _ = self.pipewire().send_message(message);

        // Wait for a response (we don't need to handle the value here)
        rx.recv()??;

        Ok(())
    }

    async fn filter_remove(&mut self, id: Ulid) -> Result<()> {
        self.filter_pw_remove(id).await
    }

    async fn filter_debug_create(&mut self, props: FilterProperties) -> Result<()> {
        self.filter_pw_create(props).await
    }

    async fn node_load_filters(&mut self, id: Ulid) -> Result<()> {
        let device_filters = self.get_device_filters_mut(id)?;

        for (index, filter) in device_filters.clone().iter().enumerate() {
            self.filter_create_custom(id, filter.clone(), index).await?;
        }

        Ok(())
    }

    async fn filter_custom_create(&mut self, target: Ulid, filter: Filter) -> Result<()> {
        let id = self.get_filter_count(target)? + 1;

        self.filter_create_custom(target, filter.clone(), id)
            .await?;
        self.add_filter_to_profile(target, filter)
    }
}

trait FilterManagementLocal {
    async fn filter_pw_create(&self, props: FilterProperties) -> Result<()>;
    async fn filter_pw_remove(&self, id: Ulid) -> Result<()>;

    fn filter_pass_get_props(&self, name: String, id: Ulid) -> FilterProperties;
    fn filter_volume_get_props(&self, name: String, id: Ulid) -> FilterProperties;
    fn filter_meter_get_props(&self, node: Ulid, name: String, id: Ulid) -> FilterProperties;

    fn get_filter_count(&mut self, target: Ulid) -> Result<usize>;
    fn add_filter_to_profile(&mut self, target: Ulid, filter: Filter) -> Result<()>;
    fn get_device_filters_mut(&mut self, target: Ulid) -> Result<&mut Vec<Filter>>;

    async fn filter_create_custom(
        &mut self,
        target: Ulid,
        filter: Filter,
        index: usize,
    ) -> Result<()>;
}

impl FilterManagementLocal for PipewireManager {
    async fn filter_pw_create(&self, mut props: FilterProperties) -> Result<()> {
        let (send, recv) = oneshot::channel();

        props.ready_sender = Some(send);
        self.pipewire()
            .send_message(PipewireMessage::CreateFilterNode(props))?;
        recv.await?;

        Ok(())
    }

    async fn filter_pw_remove(&self, id: Ulid) -> Result<()> {
        let message = PipewireMessage::RemoveFilterNode(id);
        self.pipewire().send_message(message)
    }

    fn filter_pass_get_props(&self, name: String, id: Ulid) -> FilterProperties {
        let description = name.to_lowercase().replace(" ", "-");

        FilterProperties {
            filter_id: id,
            filter_name: "Pass-Through".into(),
            filter_nick: name.to_string(),
            filter_description: format!("{}/{}", APP_NAME_ID, description),

            class: MediaClass::Duplex,
            app_id: APP_ID.to_string(),
            app_name: APP_NAME.to_string(),
            linger: false,
            callback: Box::new(PassThroughFilter::new()),

            ready_sender: None,
        }
    }

    fn filter_volume_get_props(&self, name: String, id: Ulid) -> FilterProperties {
        let description = name.to_lowercase().replace(" ", "-");

        FilterProperties {
            filter_id: id,
            filter_name: "Volume".into(),
            filter_nick: name.to_string(),
            filter_description: format!("{}/{}", APP_NAME_ID, description),

            class: MediaClass::Duplex,
            app_id: APP_ID.to_string(),
            app_name: APP_NAME.to_string(),
            linger: false,
            callback: Box::new(VolumeFilter::new(0)),

            ready_sender: None,
        }
    }

    fn filter_meter_get_props(&self, node: Ulid, name: String, id: Ulid) -> FilterProperties {
        let description = name.to_lowercase().replace(" ", "-");
        let rate = self.clock_rate.unwrap_or(48000);

        FilterProperties {
            filter_id: id,
            filter_name: "Meter".into(),
            filter_nick: name.to_string(),
            filter_description: format!("{}/{}", APP_NAME_ID, description),

            class: MediaClass::Source,
            app_id: APP_ID.to_string(),
            app_name: APP_NAME.to_string(),
            linger: false,
            callback: Box::new(MeterFilter::new(
                node,
                self.meter_callback.clone(),
                self.meter_enabled,
                rate,
            )),

            ready_sender: None,
        }
    }

    fn get_filter_count(&mut self, target: Ulid) -> Result<usize> {
        let filters = self.get_device_filters_mut(target)?;
        Ok(filters.len())
    }
    fn add_filter_to_profile(&mut self, target: Ulid, filter: Filter) -> Result<()> {
        let filters = self.get_device_filters_mut(target)?;
        filters.push(filter);
        Ok(())
    }

    fn get_device_filters_mut(&mut self, target: Ulid) -> Result<&mut Vec<Filter>> {
        let node_type = self
            .get_node_type(target)
            .ok_or_else(|| anyhow!("Target not Found"))?;

        let device_filters = match node_type {
            NodeType::PhysicalSource => {
                &mut self
                    .profile
                    .devices
                    .sources
                    .physical_devices
                    .iter_mut()
                    .find(|e| e.description.id == target)
                    .ok_or_else(|| anyhow!("Target not Found"))?
                    .filters
            }
            NodeType::PhysicalTarget => {
                &mut self
                    .profile
                    .devices
                    .targets
                    .physical_devices
                    .iter_mut()
                    .find(|e| e.description.id == target)
                    .ok_or_else(|| anyhow!("Target not Found"))?
                    .filters
            }
            NodeType::VirtualSource => {
                &mut self
                    .profile
                    .devices
                    .sources
                    .virtual_devices
                    .iter_mut()
                    .find(|e| e.description.id == target)
                    .ok_or_else(|| anyhow!("Target not Found"))?
                    .filters
            }
            NodeType::VirtualTarget => {
                &mut self
                    .profile
                    .devices
                    .targets
                    .virtual_devices
                    .iter_mut()
                    .find(|e| e.description.id == target)
                    .ok_or_else(|| anyhow!("Target not Found"))?
                    .filters
            }
        };
        Ok(device_filters)
    }

    async fn filter_create_custom(
        &mut self,
        target: Ulid,
        filter: Filter,
        index: usize,
    ) -> Result<()> {
        let node_desc = self.node_get_description(target).await?;

        // Get the number of existing filters
        //let filter_count = self.get_filter_count(target)? + 1;

        match filter.clone() {
            Filter::LV2(lv2_filter) => {
                // TODO: Check for LV2 feature support

                // This ID should be generated automatically if a ulid isn't directly provided
                let id = lv2_filter.id;
                let uri = lv2_filter.plugin_uri;

                // We need to pull the last segment of the URI for the name
                let name = uri
                    .rsplit("/")
                    .next()
                    .ok_or_else(|| anyhow!("Failed to get filter name from URI"))?;

                let name = format!("{}-{}-{}", node_desc.name, name, index);
                let defaults = HashMap::new();

                let (name, props) = filter_lv2(uri, name, id, defaults);
                self.filter_pw_create(props).await?;

                // Grab the filter parameters
                let (tx, rx) = oneshot::channel();
                let message = PipewireMessage::GetFilterParameters(id, tx);
                self.pipewire().send_message(message)?;

                let parameters = rx.recv()??;
                let config = FilterConfig { name, parameters };

                self.filter_config.insert(id, config);
            }
        }
        Ok(())
    }
}
