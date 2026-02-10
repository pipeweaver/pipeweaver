use crate::handler::pipewire::components::audio_filters::internal::meter::MeterFilter;
use crate::handler::pipewire::components::audio_filters::internal::pass_through::PassThroughFilter;
use crate::handler::pipewire::components::audio_filters::internal::volume::VolumeFilter;
use crate::handler::pipewire::components::audio_filters::lv2::filters::generic::filter_lv2;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::manager::PipewireManager;
use crate::{APP_ID, APP_NAME, APP_NAME_ID};
use anyhow::{Result, anyhow, bail};
use log::warn;
use pipeweaver_pipewire::oneshot;
use pipeweaver_pipewire::{FilterProperties, MediaClass, PipewireMessage};
use pipeweaver_profile::Filter;
use pipeweaver_shared::{FilterConfig, FilterState, FilterValue, NodeType};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use ulid::Ulid;

static FILTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
fn next_filter_id() -> usize {
    FILTER_COUNTER.fetch_add(1, Ordering::Relaxed)
}

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
    async fn filter_custom_remove(&mut self, id: Ulid) -> Result<()>;

    async fn filter_set_value(&mut self, filter: Ulid, id: u32, value: FilterValue) -> Result<()>;
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
            let defaults = match filter {
                Filter::LV2(lv2_filter) => lv2_filter.values.clone(), // Use existing values as defaults for LV2 filters
            };
            self.filter_create_custom(id, filter.clone(), index, defaults)
                .await?;
        }

        Ok(())
    }

    async fn filter_custom_create(&mut self, target: Ulid, filter: Filter) -> Result<()> {
        let id = next_filter_id();
        let defaults = HashMap::new();

        self.filter_create_custom(target, filter.clone(), id, defaults)
            .await?;
        self.add_filter_to_profile(target, filter)
    }

    async fn filter_custom_remove(&mut self, id: Ulid) -> Result<()> {
        let err = anyhow!("Filter not found in config");
        let filter = self.filter_config.remove(&id).ok_or(err)?;

        // Only remove the filter if it's running
        if filter.state == FilterState::Running {
            self.filter_pw_remove(id).await?;
        }
        self.remove_filter_from_profile(id)?;
        Ok(())
    }

    async fn filter_set_value(&mut self, filter: Ulid, id: u32, value: FilterValue) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        let message = PipewireMessage::SetFilterValue(filter, id, value.clone(), tx);
        self.pipewire().send_message(message)?;
        rx.recv()??;

        // Update the filter configuration cache
        if let Some(filter_conf) = self.filter_config.get_mut(&filter) {
            let param = filter_conf.parameters.iter_mut().find(|p| p.id == id);
            if let Some(param) = param {
                // Store the symbol for updating the profile
                let symbol = param.symbol.clone();
                param.value = value.clone();

                // Update the profile's filter entry
                let dev = self.get_device_by_filter_mut(filter)?;

                // Find this filter in the list and update based on filter type
                let filter_entry = dev.iter_mut().find(|f| match f {
                    Filter::LV2(lv2) => lv2.id == filter,
                });

                if let Some(found_filter) = filter_entry {
                    // Update the filter's parameters based on its type
                    match found_filter {
                        Filter::LV2(lv2_filter) => {
                            lv2_filter.values.insert(symbol, value);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

trait FilterManagementLocal {
    async fn filter_pw_create(&self, props: FilterProperties) -> Result<()>;
    async fn filter_pw_remove(&self, id: Ulid) -> Result<()>;

    fn filter_pass_get_props(&self, name: String, id: Ulid) -> FilterProperties;
    fn filter_volume_get_props(&self, name: String, id: Ulid) -> FilterProperties;
    fn filter_meter_get_props(&self, node: Ulid, name: String, id: Ulid) -> FilterProperties;

    fn add_filter_to_profile(&mut self, target: Ulid, filter: Filter) -> Result<()>;
    fn remove_filter_from_profile(&mut self, filter: Ulid) -> Result<()>;

    fn get_device_filters_mut(&mut self, target: Ulid) -> Result<&mut Vec<Filter>>;
    fn get_device_by_filter_mut(&mut self, filter: Ulid) -> Result<&mut Vec<Filter>>;

    async fn filter_create_custom(
        &mut self,
        target: Ulid,
        filter: Filter,
        index: usize,
        defaults: HashMap<String, FilterValue>,
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

    fn add_filter_to_profile(&mut self, target: Ulid, filter: Filter) -> Result<()> {
        let filters = self.get_device_filters_mut(target)?;
        filters.push(filter);
        Ok(())
    }
    fn remove_filter_from_profile(&mut self, filter: Ulid) -> Result<()> {
        // Find the device that contains this filter
        let device_filters = self.get_device_by_filter_mut(filter)?;

        // Remove the filter from the device's filter list
        device_filters.retain(|f| match f {
            Filter::LV2(lv2) => lv2.id != filter,
        });

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

    fn get_device_by_filter_mut(&mut self, filter: Ulid) -> Result<&mut Vec<Filter>> {
        // Define a matcher function
        let filter_matches = |f: &Filter| match f {
            Filter::LV2(lv2) => lv2.id == filter,
        };

        // This macro is a new way to reduce duplication, might extend it elsewhere later.
        macro_rules! search_devices {
            ($devices:expr) => {
                if let Some(device) = $devices
                    .iter_mut()
                    .find(|d| d.filters.iter().any(&filter_matches))
                {
                    return Ok(&mut device.filters);
                }
            };
        }

        // Search the devices :)
        search_devices!(self.profile.devices.sources.physical_devices);
        search_devices!(self.profile.devices.targets.physical_devices);
        search_devices!(self.profile.devices.sources.virtual_devices);
        search_devices!(self.profile.devices.targets.virtual_devices);

        Err(anyhow!("Filter not found in any device"))
    }

    async fn filter_create_custom(
        &mut self,
        target: Ulid,
        filter: Filter,
        index: usize,
        defaults: HashMap<String, FilterValue>,
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
                let plugin_name = uri
                    .rsplit("/")
                    .next()
                    .ok_or_else(|| anyhow!("Failed to get filter name from URI"))?;

                let name = format!("{}-{}-{}", node_desc.name, plugin_name, index);
                let create_filter = filter_lv2(uri.clone(), name.clone(), id, defaults.clone());

                // Ok, even if a filter fails to create, we still want to keep track of it in both
                // the profile and filter config, this is so we can report to the user what is
                // wrong, and correctly load next time if they correct it.
                let (name, parameters, state) = match create_filter {
                    Ok((name, props)) => {
                        // Create the filter in PipeWire
                        self.filter_pw_create(props).await?;

                        // Grab the filter parameters
                        let (tx, rx) = oneshot::channel();
                        let message = PipewireMessage::GetFilterParameters(id, tx);
                        self.pipewire().send_message(message)?;

                        (name, rx.recv()??, FilterState::Running)
                    }
                    Err(e) => {
                        warn!("Failed to create LV2 filter '{}': {}", uri, e);
                        (plugin_name.to_string(), Vec::new(), e)
                    }
                };

                let config = FilterConfig {
                    name,
                    state,
                    parameters,
                };
                self.filter_config.insert(id, config);
            }
        }
        Ok(())
    }
}
