use crate::handler::pipewire::components::audio_filters::internal::meter::MeterFilter;
use crate::handler::pipewire::components::audio_filters::internal::pass_through::PassThroughFilter;
use crate::handler::pipewire::components::audio_filters::internal::volume::VolumeFilter;
use crate::handler::pipewire::components::links::LinkManagement;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::manager::PipewireManager;
use crate::{APP_ID, APP_NAME, APP_NAME_ID};
use anyhow::{Result, anyhow, bail};
use log::warn;
use pipeweaver_pipewire::oneshot;
use pipeweaver_pipewire::{FilterProperties, MediaClass, PipewireMessage};
use pipeweaver_profile::{Filter, FilterType};
use pipeweaver_shared::{FilterConfig, FilterState, FilterValue, Mix, NodeType};
use std::collections::{HashMap, HashSet};
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

    async fn channel_load_filters(&mut self, id: Ulid) -> Result<()>;
    async fn source_link_to_filters(&mut self, id: Ulid, is_node: bool) -> Result<()>;

    async fn filter_custom_create(&mut self, target: Ulid, filter: Filter) -> Result<()>;
    async fn filter_custom_remove(&mut self, id: Ulid) -> Result<()>;
    async fn filter_custom_move(&mut self, id: Ulid, new_index: usize) -> Result<()>;

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

    async fn channel_load_filters(&mut self, id: Ulid) -> Result<()> {
        if self.get_device_filters(id)?.is_empty() {
            return Ok(());
        }

        // Create a passthrough filter which we'll use as either the start / end point for the
        // filter tree.
        let pass = self.filter_pass_create(String::from("FilterTree")).await?;
        let node_type = self
            .get_node_type(id)
            .ok_or_else(|| anyhow!("Node not Found"))?;
        match node_type {
            NodeType::PhysicalSource | NodeType::VirtualSource => {
                // Flag the pass-through filter as the end of the tree
                self.source_filter_end.insert(id, pass);
            }
            NodeType::PhysicalTarget | NodeType::VirtualTarget => {}
        }

        let mut previous_filter_id = None;
        let device_filters = self.get_device_filters(id)?.clone();
        for (index, filter) in device_filters.into_iter().enumerate() {
            let filter_id = filter.id;

            let defaults = match &filter.filter {
                FilterType::LV2(lv2_filter) => lv2_filter.values.clone(),
            };

            self.filter_create_custom(id, filter, index, defaults)
                .await?;

            // Make sure this filter was created and is running
            if let Some(filter_config) = self.filter_config.get(&filter_id)
                && filter_config.state == FilterState::Running
            {
                if let Some(prev_filter_id) = previous_filter_id {
                    self.link_create_filter_to_filter(prev_filter_id, filter_id)
                        .await?;
                }
                previous_filter_id = Some(filter_id);
            }
        }

        // Link the last filter to the pass-through filter
        if let Some(prev_filter_id) = previous_filter_id {
            self.link_create_filter_to_filter(prev_filter_id, pass)
                .await?;
        }

        Ok(())
    }

    async fn source_link_to_filters(&mut self, id: Ulid, is_node: bool) -> Result<()> {
        let filter_ids: Vec<Ulid> = self
            .get_device_filters(id)?
            .iter()
            .map(|filter| filter.id)
            .collect();

        for filter_id in filter_ids {
            let Some(filter_config) = self.filter_config.get(&filter_id) else {
                warn!("Filter not found in config");
                continue;
            };

            if filter_config.state == FilterState::Running {
                if is_node {
                    self.link_create_node_to_filter(id, filter_id).await?;
                } else {
                    self.link_create_filter_to_filter(id, filter_id).await?;
                }
                return Ok(());
            }
        }

        // If we get here, we didn't find any running filters, so we'll just link to the passthrough
        let pass_id = self
            .source_filter_end
            .get(&id)
            .expect("No pass-through filter found for source");

        if is_node {
            self.link_create_node_to_filter(id, *pass_id).await?;
        } else {
            self.link_create_filter_to_filter(id, *pass_id).await?;
        }

        Ok(())
    }

    async fn filter_custom_create(&mut self, target: Ulid, filter: Filter) -> Result<()> {
        let id = next_filter_id();
        let defaults = HashMap::new();

        // Find the last running filter in this tree
        let last_running = {
            let running: HashSet<Ulid> = self
                .filter_config
                .iter()
                .filter_map(|(&fid, cfg)| (cfg.state == FilterState::Running).then_some(fid))
                .collect();

            let device_filters = self.get_device_filters(target)?;

            device_filters
                .iter()
                .rev()
                .find(|f| running.contains(&f.id))
                .map(|f| f.id)
        };

        // Create the new filter
        self.filter_create_custom(target, filter.clone(), id, defaults)
            .await?;

        let is_running = self
            .filter_config
            .get(&filter.id)
            .map(|cfg| cfg.state == FilterState::Running)
            .ok_or_else(|| anyhow!("Filter {id} not found in config after creation"))?;

        if !is_running {
            self.add_filter_to_profile(target, filter)?;
            return Ok(());
        }

        //let (device_id, node_type) = self.get_device_id_by_filter(target)?;
        let device_id = target;
        let node_type = self
            .get_node_type(target)
            .ok_or_else(|| anyhow!("Node not Found"))?;

        // Ensure the pass-through exists, creating it if needed
        let pass_id = match self.source_filter_end.get(&device_id).copied() {
            Some(existing) => existing,
            None => {
                let pass_id = self.filter_pass_create(String::from("FilterTree")).await?;
                self.source_filter_end.insert(device_id, pass_id);

                match node_type {
                    NodeType::PhysicalSource | NodeType::VirtualSource => {
                        // We need to attach the pass-through to the volume / meter ports..
                        let meter = self.meter_map[&target];

                        if self.meter_enabled {
                            self.link_create_filter_to_filter(pass_id, meter).await?;
                        }

                        // Now we need to link our node to the Mixes
                        let mix_a = self.source_map[&target][Mix::A];
                        let mix_b = self.source_map[&target][Mix::B];

                        self.link_create_filter_to_filter(pass_id, mix_a).await?;
                        self.link_create_filter_to_filter(pass_id, mix_b).await?;

                        // Now we need to drop the original links
                        if node_type == NodeType::VirtualSource {
                            if self.meter_enabled {
                                self.link_remove_node_to_filter(device_id, meter).await?;
                            }
                            self.link_remove_node_to_filter(device_id, mix_a).await?;
                            self.link_remove_node_to_filter(device_id, mix_b).await?;
                        } else {
                            if self.meter_enabled {
                                self.link_remove_filter_to_filter(device_id, meter).await?;
                            }
                            self.link_remove_filter_to_filter(device_id, mix_a).await?;
                            self.link_remove_filter_to_filter(device_id, mix_b).await?;
                        }
                    }
                    _ => {
                        // Not implemented yet
                    }
                }

                pass_id
            }
        };

        // Detach whatever is currently linked to the pass-through
        match last_running {
            Some(last_id) => self.link_remove_filter_to_filter(last_id, pass_id).await?,
            None => match node_type {
                NodeType::PhysicalSource => {
                    self.link_remove_filter_to_filter(device_id, pass_id)
                        .await?
                }
                NodeType::VirtualSource => {
                    self.link_remove_node_to_filter(device_id, pass_id).await?
                }

                // TODO: Not supported yet :D
                NodeType::PhysicalTarget | NodeType::VirtualTarget => {}
            },
        }

        // Insert the new filter before the pass-through
        match last_running {
            Some(last_id) => {
                self.link_create_filter_to_filter(last_id, filter.id)
                    .await?
            }
            None => match node_type {
                NodeType::PhysicalSource => {
                    self.link_create_filter_to_filter(device_id, filter.id)
                        .await?
                }
                NodeType::VirtualSource => {
                    self.link_create_node_to_filter(device_id, filter.id)
                        .await?
                }

                // TODO: Not supported yet :D
                NodeType::PhysicalTarget | NodeType::VirtualTarget => {}
            },
        }

        self.link_create_filter_to_filter(filter.id, pass_id)
            .await?;
        self.add_filter_to_profile(target, filter)?;
        Ok(())
    }

    async fn filter_custom_remove(&mut self, id: Ulid) -> Result<()> {
        let err = anyhow!("Filter not found in config");
        let filter = self.filter_config.remove(&id).ok_or(err)?;

        // If this filter isn't running, we just need to remove it from the profile
        if filter.state != FilterState::Running {
            self.remove_filter_from_profile(id)?;
            return Ok(());
        }

        // Get the device, and the previous and next running filters
        let (device_id, node_type) = self.get_device_id_by_filter(id)?;
        let (prev, next) = self.find_running_neighbours(id)?;

        // For sources, find the pass-through filter at the end of the chain
        // (only relevant when the removed filter has no next running neighbour)
        let source_pass_filter = matches!(
            node_type,
            NodeType::PhysicalSource | NodeType::VirtualSource
        )
        .then(|| {
            self.source_filter_end
                .get(&device_id)
                .copied()
                .filter(|&pass_id| pass_id != id)
        })
        .flatten();

        // Remove the incoming link
        match (prev, node_type) {
            (Some(prev_id), _) => self.link_remove_filter_to_filter(prev_id, id).await?,
            (None, NodeType::PhysicalSource) => {
                self.link_remove_filter_to_filter(device_id, id).await?
            }
            (None, NodeType::VirtualSource) => {
                self.link_remove_node_to_filter(device_id, id).await?
            }
            (None, NodeType::PhysicalTarget | NodeType::VirtualTarget) => {}
        }

        // Remove the outgoing link
        match (next, source_pass_filter) {
            (Some(next_id), _) => self.link_remove_filter_to_filter(id, next_id).await?,
            (None, Some(pass_id)) => self.link_remove_filter_to_filter(id, pass_id).await?,
            (None, None) => {}
        }

        // Bridge across the outgoing filter
        match (prev, next, source_pass_filter) {
            // We have a previous and next filter, so link between the two
            (Some(prev_id), Some(next_id), _) => {
                self.link_create_filter_to_filter(prev_id, next_id).await?;
            }

            // We don't have a next filter, so link to the pass-through
            (Some(prev_id), None, Some(pass_id)) => {
                self.link_create_filter_to_filter(prev_id, pass_id).await?;
            }

            // We don't have a previous filter, so link from source node
            (None, Some(next_id), _) => match node_type {
                NodeType::PhysicalSource => {
                    self.link_create_filter_to_filter(device_id, next_id)
                        .await?
                }
                NodeType::VirtualSource => {
                    self.link_create_node_to_filter(device_id, next_id).await?
                }

                // TODO: Not supported yet :D
                NodeType::PhysicalTarget | NodeType::VirtualTarget => {}
            },

            // We don't have a previous or next filter, so link source to the pass-through
            (None, None, Some(pass_id)) => match node_type {
                NodeType::PhysicalSource => {
                    self.link_create_filter_to_filter(device_id, pass_id)
                        .await?
                }
                NodeType::VirtualSource => {
                    self.link_create_node_to_filter(device_id, pass_id).await?
                }

                // TODO: Not supported yet :D
                NodeType::PhysicalTarget | NodeType::VirtualTarget => {}
            },
            _ => {
                bail!("Unexpected filter chain");
            }
        }

        // Remove the filter from pipewire
        self.filter_pw_remove(id).await?;

        // Remove the filter from the profile
        self.remove_filter_from_profile(id)?;

        // And we should be done :)
        Ok(())
    }

    async fn filter_custom_move(&mut self, id: Ulid, new_index: usize) -> Result<()> {
        // Only bother if the filter is running
        let is_running = self
            .filter_config
            .get(&id)
            .map(|cfg| cfg.state == FilterState::Running)
            .ok_or_else(|| anyhow!("Filter {id} not found in config"))?;

        let (device_id, node_type) = self.get_device_id_by_filter(id)?;

        // Grab neighbours before the move
        let (old_prev, old_next) = self.find_running_neighbours(id)?;

        // Reorder the vec
        {
            let device_filters = self.get_device_filters_mut(device_id)?;
            let old_index = device_filters
                .iter()
                .position(|f| f.id == id)
                .ok_or_else(|| anyhow!("Filter {id} not found in device chain"))?;

            let filter = device_filters.remove(old_index);
            let clamped = new_index.min(device_filters.len());
            device_filters.insert(clamped, filter);
        }

        if !is_running {
            return Ok(());
        }

        // Grab neighbours after the move
        let (new_prev, new_next) = self.find_running_neighbours(id)?;

        // If nothing changed in terms of running neighbours, no relink needed
        if old_prev == new_prev && old_next == new_next {
            return Ok(());
        }

        let source_pass_filter = matches!(
            node_type,
            NodeType::PhysicalSource | NodeType::VirtualSource
        )
        .then(|| self.source_filter_end.get(&device_id).copied())
        .flatten();

        // Remove incoming link to filter
        match (old_prev, node_type) {
            (Some(prev_id), _) => self.link_remove_filter_to_filter(prev_id, id).await?,
            (None, NodeType::PhysicalSource) => {
                self.link_remove_filter_to_filter(device_id, id).await?
            }
            (None, NodeType::VirtualSource) => {
                self.link_remove_node_to_filter(device_id, id).await?
            }
            (None, NodeType::PhysicalTarget | NodeType::VirtualTarget) => {}
        }

        // Remove outgoing link from filter
        match (old_next, source_pass_filter) {
            (Some(next_id), _) => self.link_remove_filter_to_filter(id, next_id).await?,
            (None, Some(pass_id)) => self.link_remove_filter_to_filter(id, pass_id).await?,
            (None, None) => {
                bail!("Failed to remove old outgoing link (next and pass are None)");
            }
        }

        // Bridge the old gap
        match (old_prev, old_next, source_pass_filter) {
            (Some(prev_id), Some(next_id), _) => {
                self.link_create_filter_to_filter(prev_id, next_id).await?;
            }
            (Some(prev_id), None, Some(pass_id)) => {
                self.link_create_filter_to_filter(prev_id, pass_id).await?;
            }
            (None, Some(next_id), _) => match node_type {
                NodeType::PhysicalSource => {
                    self.link_create_filter_to_filter(device_id, next_id)
                        .await?
                }
                NodeType::VirtualSource => {
                    self.link_create_node_to_filter(device_id, next_id).await?
                }
                NodeType::PhysicalTarget | NodeType::VirtualTarget => {}
            },
            (None, None, Some(pass_id)) => match node_type {
                NodeType::PhysicalSource => {
                    self.link_create_filter_to_filter(device_id, pass_id)
                        .await?
                }
                NodeType::VirtualSource => {
                    self.link_create_node_to_filter(device_id, pass_id).await?
                }
                NodeType::PhysicalTarget | NodeType::VirtualTarget => {}
            },
            _ => {
                bail!("Failed to bridge old link (prev, next and pass are None)");
            }
        }

        // Remove whatever link currently bridges the new gap
        match (new_prev, new_next, source_pass_filter) {
            (Some(prev_id), Some(next_id), _) => {
                self.link_remove_filter_to_filter(prev_id, next_id).await?;
            }
            (Some(prev_id), None, Some(pass_id)) => {
                self.link_remove_filter_to_filter(prev_id, pass_id).await?;
            }
            (None, Some(next_id), _) => match node_type {
                NodeType::PhysicalSource => {
                    self.link_remove_filter_to_filter(device_id, next_id)
                        .await?
                }
                NodeType::VirtualSource => {
                    self.link_remove_node_to_filter(device_id, next_id).await?
                }
                NodeType::PhysicalTarget | NodeType::VirtualTarget => {}
            },
            (None, None, Some(pass_id)) => match node_type {
                NodeType::PhysicalSource => {
                    self.link_remove_filter_to_filter(device_id, pass_id)
                        .await?
                }
                NodeType::VirtualSource => {
                    self.link_remove_node_to_filter(device_id, pass_id).await?
                }
                NodeType::PhysicalTarget | NodeType::VirtualTarget => {}
            },
            _ => {
                bail!("Failed to remove outgoing link (prev, next and pass are None)");
            }
        }

        // Create incoming link in new position
        match (new_prev, node_type) {
            (Some(prev_id), _) => self.link_create_filter_to_filter(prev_id, id).await?,
            (None, NodeType::PhysicalSource) => {
                self.link_create_filter_to_filter(device_id, id).await?
            }
            (None, NodeType::VirtualSource) => {
                self.link_create_node_to_filter(device_id, id).await?
            }
            (None, NodeType::PhysicalTarget | NodeType::VirtualTarget) => {}
        }

        // Create outgoing link in new position
        match (new_next, source_pass_filter) {
            (Some(next_id), _) => self.link_create_filter_to_filter(id, next_id).await?,
            (None, Some(pass_id)) => self.link_create_filter_to_filter(id, pass_id).await?,
            (None, None) => {
                bail!("Failed to create outgoing link");
            }
        }

        Ok(())
    }

    async fn filter_set_value(&mut self, filter: Ulid, id: u32, value: FilterValue) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        let message = PipewireMessage::SetFilterValue(filter, id, value.clone(), tx);
        self.pipewire().send_message(message)?;
        rx.recv()??;

        // Update the filter configuration cache
        if let Some(filter_conf) = self.filter_config.get_mut(&filter)
            && let Some(param) = filter_conf.parameters.iter_mut().find(|p| p.id == id)
        {
            // Store the symbol for updating the profile
            let symbol = param.symbol.clone();
            param.value = value.clone();

            // Update the profile's filter entry
            let dev = self.get_device_by_filter_mut(filter)?;

            // Find this filter in the list and update based on filter type
            if let Some(found_filter) = dev.iter_mut().find(|f| f.id == filter) {
                // Update the filter's parameters based on its type
                match &mut found_filter.filter {
                    FilterType::LV2(lv2_filter) => {
                        lv2_filter.values.insert(symbol, value);
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

    fn get_device_filters(&self, target: Ulid) -> Result<&Vec<Filter>>;
    fn get_device_filters_mut(&mut self, target: Ulid) -> Result<&mut Vec<Filter>>;
    fn get_device_by_filter_mut(&mut self, filter: Ulid) -> Result<&mut Vec<Filter>>;
    fn get_device_id_by_filter(&self, filter: Ulid) -> Result<(Ulid, NodeType)>;
    fn find_running_neighbours(&mut self, id: Ulid) -> Result<(Option<Ulid>, Option<Ulid>)>;

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
        device_filters.retain(|f| f.id != filter);

        Ok(())
    }

    fn get_device_filters(&self, target: Ulid) -> Result<&Vec<Filter>> {
        let node_type = self
            .get_node_type(target)
            .ok_or_else(|| anyhow!("Target not Found"))?;

        macro_rules! find_device_filters {
            ($devices:expr) => {
                $devices
                    .iter()
                    .find(|device| device.description.id == target)
                    .map(|device| &device.filters)
                    .ok_or_else(|| anyhow!("Target not Found"))
            };
        }

        match node_type {
            NodeType::PhysicalSource => {
                find_device_filters!(self.profile.devices.sources.physical_devices)
            }
            NodeType::PhysicalTarget => {
                find_device_filters!(self.profile.devices.targets.physical_devices)
            }
            NodeType::VirtualSource => {
                find_device_filters!(self.profile.devices.sources.virtual_devices)
            }
            NodeType::VirtualTarget => {
                find_device_filters!(self.profile.devices.targets.virtual_devices)
            }
        }
    }

    fn get_device_filters_mut(&mut self, target: Ulid) -> Result<&mut Vec<Filter>> {
        let node_type = self
            .get_node_type(target)
            .ok_or_else(|| anyhow!("Target not Found"))?;

        macro_rules! find_device_filters {
            ($devices:expr) => {
                $devices
                    .iter_mut()
                    .find(|device| device.description.id == target)
                    .map(|device| &mut device.filters)
                    .ok_or_else(|| anyhow!("Target not Found"))
            };
        }

        match node_type {
            NodeType::PhysicalSource => {
                find_device_filters!(self.profile.devices.sources.physical_devices)
            }
            NodeType::PhysicalTarget => {
                find_device_filters!(self.profile.devices.targets.physical_devices)
            }
            NodeType::VirtualSource => {
                find_device_filters!(self.profile.devices.sources.virtual_devices)
            }
            NodeType::VirtualTarget => {
                find_device_filters!(self.profile.devices.targets.virtual_devices)
            }
        }
    }

    fn get_device_by_filter_mut(&mut self, filter: Ulid) -> Result<&mut Vec<Filter>> {
        // Define a matcher function
        let filter_matches = |f: &Filter| f.id == filter;

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

    fn get_device_id_by_filter(&self, filter: Ulid) -> Result<(Ulid, NodeType)> {
        let filter_matches = |f: &Filter| f.id == filter;

        macro_rules! search_devices {
            ($devices:expr, $node_type:expr) => {
                if let Some(device) = $devices
                    .iter()
                    .find(|d| d.filters.iter().any(&filter_matches))
                {
                    return Ok((device.description.id, $node_type));
                }
            };
        }

        search_devices!(
            self.profile.devices.sources.physical_devices,
            NodeType::PhysicalSource
        );
        search_devices!(
            self.profile.devices.targets.physical_devices,
            NodeType::PhysicalTarget
        );
        search_devices!(
            self.profile.devices.sources.virtual_devices,
            NodeType::VirtualSource
        );
        search_devices!(
            self.profile.devices.targets.virtual_devices,
            NodeType::VirtualTarget
        );

        Err(anyhow!("Filter not found in any device"))
    }

    fn find_running_neighbours(&mut self, id: Ulid) -> Result<(Option<Ulid>, Option<Ulid>)> {
        let running: HashSet<Ulid> = self
            .filter_config
            .iter()
            .filter_map(|(&fid, cfg)| (cfg.state == FilterState::Running).then_some(fid))
            .collect();

        let device_filters = self.get_device_by_filter_mut(id)?;

        let idx = device_filters
            .iter()
            .position(|f| f.id == id)
            .ok_or_else(|| anyhow!("Filter {id} not found in device chain"))?;

        let prev = device_filters[..idx]
            .iter()
            .rev()
            .find(|f| running.contains(&f.id))
            .map(|f| f.id);

        let next = device_filters[idx + 1..]
            .iter()
            .find(|f| running.contains(&f.id))
            .map(|f| f.id);

        Ok((prev, next))
    }

    // We allow unused variables here, because if the LV2 feature isn't enabled, they won't be used,
    #[allow(unused_variables)]
    async fn filter_create_custom(
        &mut self,
        target: Ulid,
        filter: Filter,
        index: usize,
        defaults: HashMap<String, FilterValue>,
    ) -> Result<()> {
        // Get the number of existing filters
        //let filter_count = self.get_filter_count(target)? + 1;

        match filter.filter {
            FilterType::LV2(lv2_filter) => {
                // This ID should be generated automatically if a ulid isn't directly provided
                let id = filter.id;
                let uri = lv2_filter.plugin_uri;

                // We need to pull the last segment of the URI for the name
                let plugin_name = uri
                    .rsplit("/")
                    .next()
                    .ok_or_else(|| anyhow!("Failed to get filter name from URI"))?;

                #[cfg(feature = "lv2")]
                {
                    use log::warn;
                    use crate::handler::pipewire::components::audio_filters::lv2::filters::generic::filter_lv2;
                    let node_desc = self.node_get_description(target).await?;

                    let name = format!("{}-{}-{}", node_desc.name, plugin_name, index);
                    let rate = self.clock_rate.unwrap_or(48000);
                    let create_filter =
                        filter_lv2(uri.clone(), name.clone(), id, defaults.clone(), rate);

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
                        identifier: uri,
                        state,
                        parameters,
                    };
                    self.filter_config.insert(id, config);
                }
                #[cfg(not(feature = "lv2"))]
                {
                    let config = FilterConfig {
                        name: plugin_name.to_string(),
                        state: FilterState::FeatureMissing("lv2".to_string()),
                        parameters: Vec::new(),
                    };
                    self.filter_config.insert(id, config);
                }
            }
        }
        Ok(())
    }
}
