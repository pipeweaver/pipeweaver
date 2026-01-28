use crate::handler::pipewire::components::audio_filters::lv2::base::{
    ControlConvert, LV2PluginBase,
};
use crate::{APP_ID, APP_NAME, APP_NAME_ID};
use log::{debug, warn};
use pipeweaver_pipewire::{
    FilterHandler, FilterProperties, FilterProperty, FilterValue, MediaClass,
};
use std::collections::HashMap;
use ulid::Ulid;

// This is a helper enum to determine port types, they abstractly map to FilterValue variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PortType {
    Bool,
    Int,
    Float,
}

/// A generic LV2 filter that can handle any LV2 plugin by dynamically discovering its control ports.
pub struct GenericLV2Filter {
    plugin: LV2PluginBase,

    // Runtime storage for control port values
    // Maps property_id -> current value
    control_values: HashMap<u32, f32>,

    // Maps property_id -> port symbol for quick lookup
    id_to_symbol: HashMap<u32, String>,

    // Maps port symbol -> property_id for reverse lookup
    symbol_to_id: HashMap<String, u32>,
}

impl GenericLV2Filter {
    #[inline]
    fn set_and_cache<T>(&mut self, symbol: &str, id: u32, value: T)
    where
        T: ControlConvert + std::fmt::Debug + Clone,
    {
        self.plugin.set_typed_prop(symbol, value, |val| {
            self.control_values.insert(id, val.to_f32());
        });
    }
}

impl FilterHandler for GenericLV2Filter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        let mut props = Vec::new();

        // Iterate through all control ports in order of property ID
        let mut ids: Vec<_> = self.id_to_symbol.keys().copied().collect();
        ids.sort();

        for id in ids {
            props.push(self.get_property(id));
        }

        props
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        let symbol = self.id_to_symbol.get(&id).expect("Invalid property ID");

        let port = self
            .plugin
            .control_ports
            .get(symbol)
            .expect("Port not found for symbol");

        let current_value = self
            .control_values
            .get(&id)
            .copied()
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
            symbol: symbol.clone(),
            value,
            min: port.min,
            max: port.max,
            enum_def: port.is_enum.clone(),
        }
    }

    fn set_property(&mut self, id: u32, value: FilterValue) {
        // Look up symbol and port info first, we can use this for later handling
        let (symbol, port_type) = match self.id_to_symbol.get(&id) {
            Some(s) => {
                let symbol = s.clone();
                match self.plugin.control_ports.get(&symbol) {
                    Some(p) => {
                        // Determine port type from metadata
                        let port_type = if p.is_toggled {
                            PortType::Bool
                        } else if p.is_integer || p.is_enum.is_some() {
                            PortType::Int
                        } else {
                            PortType::Float
                        };
                        (symbol, port_type)
                    }
                    None => {
                        warn!("Port not found for symbol: {}", symbol);
                        return;
                    }
                }
            }
            None => {
                warn!("Attempted to set property with unknown id: {}", id);
                return;
            }
        };

        // Now we can safely mutably borrow self since symbol is owned
        match value {
            FilterValue::Bool(v) => {
                if matches!(port_type, PortType::Bool) {
                    self.set_and_cache(&symbol, id, v);
                } else {
                    warn!("Attempted to set non-toggled port {} with bool", symbol);
                }
            }
            FilterValue::Int32(v) => {
                if matches!(port_type, PortType::Int) {
                    self.set_and_cache(&symbol, id, v);
                } else {
                    warn!("Attempted to set non-integer port {} with int32", symbol);
                }
            }
            FilterValue::Float32(v) => {
                if matches!(port_type, PortType::Float) {
                    self.set_and_cache(&symbol, id, v);
                } else {
                    warn!(
                        "Attempted to set integer/toggle port {} with float32",
                        symbol
                    );
                }
            }
            _ => {
                warn!("Unsupported FilterValue variant for port {}", symbol);
            }
        }
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, outputs: Vec<&mut [f32]>) {
        self.plugin.process(inputs, outputs);
    }
}

impl GenericLV2Filter {
    // Creates a general filter, I'm not sure if this needs to have an Option for defaults,
    // we could simply accept a bare hashmap and handle that instead.
    pub fn new(
        plugin_uri: &str,
        rate: u32,
        block_size: usize,
        defaults: HashMap<String, FilterValue>,
    ) -> Result<Self, String> {
        let plugin = LV2PluginBase::new(plugin_uri, rate, block_size)?;

        // Build bidirectional mappings between property IDs and port symbols
        // We assign property IDs sequentially based on the order of control ports
        let mut id_to_symbol = HashMap::new();
        let mut symbol_to_id = HashMap::new();
        let mut control_values = HashMap::new();

        // Sort control ports by index to ensure consistent property ID assignment
        let mut ports: Vec<_> = plugin.control_ports.iter().collect();
        ports.sort_by_key(|(_, port)| port.index);

        for (property_id, (symbol, port)) in ports.iter().enumerate() {
            let property_id = property_id as u32;
            id_to_symbol.insert(property_id, symbol.to_string());
            symbol_to_id.insert(symbol.to_string(), property_id);

            // Read the default from the plugin, or use the port default if unavailable
            let current_value = plugin.get_control_by_symbol(symbol).unwrap_or(port.default);
            control_values.insert(property_id, current_value);
        }

        let mut filter = Self {
            plugin,
            control_values,
            id_to_symbol,
            symbol_to_id,
        };

        // Apply defaults if provided
        for (symbol, value) in defaults {
            if let Some(&id) = filter.symbol_to_id.get(&symbol) {
                filter.set_property(id, value);
            } else {
                warn!("Default provided for unknown port symbol: '{}'", symbol);
            }
        }

        Ok(filter)
    }
}

/// Helper function to create FilterProperties for a generic LV2 plugin
pub fn filter_get_generic_lv2_props(
    plugin_uri: impl Into<String>,
    name: String,
    id: Ulid,
    defaults: HashMap<String, FilterValue>,
) -> FilterProperties {
    let plugin_uri = plugin_uri.into();
    let description = name.to_lowercase().replace(" ", "-");

    // Use the last segment of the URI as a unique filter identifier for PipeWire
    let filter_name = plugin_uri
        .split('/')
        .next_back()
        .unwrap_or("generic_lv2")
        .to_string();

    debug!("Filter Name: {}", filter_name);

    let callback = match GenericLV2Filter::new(&plugin_uri, 96000, 1024, defaults) {
        Ok(filter) => filter,
        Err(e) => {
            panic!(
                "Failed to create GenericLV2Filter for '{}': {}",
                plugin_uri, e
            );
        }
    };

    FilterProperties {
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
    }
}
