use crate::handler::pipewire::components::audio_filters::lv2::base::LV2PluginBase;
use crate::{APP_ID, APP_NAME, APP_NAME_ID};
use anyhow::{Result, bail};
use log::{debug, warn};
use pipeweaver_pipewire::{FilterHandler, FilterProperties, MediaClass};
use pipeweaver_shared::{FilterProperty, FilterState, FilterValue};
use std::collections::HashMap;
use ulid::Ulid;

/// A generic LV2 filter that can handle any LV2 plugin by dynamically discovering its control ports.
pub struct GenericLV2Filter {
    plugin: LV2PluginBase,

    // The Name of the Filter
    plugin_name: String,

    // Maps property_id -> port_index for quick lookups in the PluginBase
    property_to_port: Vec<u32>,
}

impl FilterHandler for GenericLV2Filter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        let mut props = Vec::with_capacity(self.property_to_port.len());

        // Iterate through all control ports in order of property ID
        for id in 0..self.property_to_port.len() {
            props.push(self.get_property(id as u32));
        }

        props
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        // Use the ID to look up the corresponding control port index
        let port_index = self.property_to_port[id as usize];
        let port = self.plugin.control_ports[port_index as usize]
            .as_ref()
            .expect("Property maps to non-control port");

        let current_value = self
            .plugin
            .get_control_by_index(port_index)
            .unwrap_or(port.default);

        // Determine the appropriate FilterValue variant based on port metadata
        let value = if port.is_toggled {
            FilterValue::Bool(current_value > 0.5)
        } else if port.is_integer || port.is_enum.is_some() {
            FilterValue::Int32(current_value as i32)
        } else {
            FilterValue::Float32(current_value)
        };

        FilterProperty {
            id,
            name: port.name.clone(),
            symbol: port.symbol.clone(),
            value,
            min: port.min,
            max: port.max,
            is_input: port.is_input,
            enum_def: port.is_enum.clone(),
        }
    }

    fn set_property(&mut self, id: u32, value: FilterValue) -> Result<String> {
        // Again, ID to find the control port index
        if id as usize >= self.property_to_port.len() {
            bail!("Attempted to set property with invalid id: {}", id);
        }

        let port_index = self.property_to_port[id as usize];

        // Get port info (no duplication - reading from plugin's control_ports)
        let port = match &self.plugin.control_ports[port_index as usize] {
            Some(p) => p,
            None => {
                bail!(
                    "Property {} maps to non-control port index {}",
                    id,
                    port_index
                );
            }
        };

        // Extract metadata so we can type-check the internal value
        let is_integer = port.is_integer;
        let is_toggled = port.is_toggled;
        let is_enum = port.is_enum.is_some();

        // Clone the Symbol for return, and errors
        let symbol = port.symbol.clone();

        // Run the value check
        match value {
            FilterValue::Bool(v) => {
                if is_toggled {
                    self.plugin.set_control_by_index_typed(port_index, v);
                } else {
                    bail!("Attempted to set non-toggled port {} with bool", symbol);
                }
            }
            FilterValue::Int32(v) => {
                if is_integer || is_enum {
                    self.plugin.set_control_by_index_typed(port_index, v);
                } else {
                    bail!("Attempted to set non-integer port {} with int32", symbol);
                }
            }
            FilterValue::Float32(v) => {
                if !is_integer && !is_toggled {
                    self.plugin.set_control_by_index_typed(port_index, v);
                } else {
                    bail!(
                        "Attempted to set integer/toggle port {} with float32",
                        symbol
                    );
                }
            }
            _ => {
                bail!("Unsupported FilterValue variant for port {}", symbol);
            }
        }
        Ok(symbol.clone())
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, outputs: Vec<&mut [f32]>) {
        self.plugin.process(inputs, outputs);
    }
}

impl GenericLV2Filter {
    /// Helper: Convert symbol to property_id using the plugin's symbol_to_port mapping
    fn symbol_to_property_id(&self, symbol: &str) -> Option<u32> {
        let port_index = self.plugin.symbol_to_port.get(symbol)?;

        self.property_to_port
            .iter()
            .position(|&idx| idx == *port_index)
            .map(|pos| pos as u32)
    }

    // Creates a general filter, defaults can be an empty HashMap if we don't want to override values
    pub fn new(
        plugin_uri: &str,
        rate: u32,
        block_size: usize,
        defaults: HashMap<String, FilterValue>,
    ) -> Result<Self, FilterState> {
        let plugin = LV2PluginBase::new(plugin_uri, rate, block_size)?;
        let plugin_name = plugin.plugin_name.clone();

        let mut property_to_port = Vec::new();

        // Collect control ports and sort by port index for consistent property ID assignment
        let mut control_port_indices: Vec<u32> = plugin
            .control_ports
            .iter()
            .enumerate()
            .filter_map(|(idx, opt)| opt.as_ref().map(|_| idx as u32))
            .collect();
        control_port_indices.sort();

        // Build the property_id -> port_index mapping
        for port_idx in control_port_indices {
            property_to_port.push(port_idx);
        }

        let mut filter = Self {
            plugin,
            plugin_name,
            property_to_port,
        };

        // Apply defaults if provided, this performs a symbol -> property_id -> port_index lookup,
        // it's a little excessive, but is only done on initialization, and allows us to just call
        // .set_property which handles shit like type checking.
        for (symbol, value) in defaults {
            if let Some(id) = filter.symbol_to_property_id(&symbol) {
                let _ = filter.set_property(id, value);
            } else {
                warn!("Default provided for unknown port symbol: '{}'", symbol);
            }
        }

        Ok(filter)
    }
}

/// Helper function to create FilterProperties for a generic LV2 plugin
pub fn filter_lv2(
    plugin_uri: impl Into<String>,
    name: String,
    id: Ulid,
    defaults: HashMap<String, FilterValue>,
) -> Result<(String, FilterProperties), FilterState> {
    let plugin_uri = plugin_uri.into();
    let description = name.to_lowercase().replace(" ", "-");

    // Use the last segment of the URI as a unique filter identifier for PipeWire
    let filter_name = plugin_uri
        .split('/')
        .next_back()
        .unwrap_or("generic_lv2")
        .to_string();

    debug!("Filter Name: {}", filter_name);

    let callback = GenericLV2Filter::new(&plugin_uri, 96000, 1024, defaults)?;
    let name = callback.plugin_name.clone();

    let props = FilterProperties {
        filter_id: id,
        filter_name,
        filter_nick: name.to_string(),
        filter_description: format!("{}/{}", APP_NAME_ID, description),

        class: MediaClass::Duplex,
        app_id: APP_ID.to_string(),
        app_name: APP_NAME.to_string(),
        linger: false,

        callback: Box::new(callback),
        ready_sender: None,
    };

    Ok((name, props))
}
