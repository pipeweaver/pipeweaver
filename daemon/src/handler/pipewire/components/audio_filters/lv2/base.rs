use lilv_sys::{
    LilvInstance, LilvNode, LilvPlugin, LilvPlugins, LilvWorld, lilv_instance_activate,
    lilv_instance_connect_port, lilv_instance_deactivate, lilv_instance_free, lilv_instance_run,
    lilv_new_uri, lilv_node_as_float, lilv_node_as_int, lilv_node_as_string, lilv_node_free,
    lilv_nodes_begin, lilv_nodes_free, lilv_nodes_get, lilv_nodes_get_first, lilv_nodes_is_end,
    lilv_nodes_next, lilv_nodes_size, lilv_plugin_get_name, lilv_plugin_get_num_ports,
    lilv_plugin_get_port_by_index, lilv_plugin_instantiate, lilv_plugins_get_by_uri,
    lilv_port_get_name, lilv_port_get_range, lilv_port_get_symbol, lilv_port_get_value,
    lilv_port_has_property, lilv_port_is_a, lilv_world_find_nodes, lilv_world_free,
    lilv_world_get_all_plugins, lilv_world_load_all, lilv_world_new,
};
use log::{debug, warn};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fmt::Debug;
use std::os::raw::c_void;
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};

// Shared LV2 world (initialized once, shared by all plugins)
static LV2_WORLD: OnceLock<Arc<Mutex<LV2World>>> = OnceLock::new();

fn get_world() -> &'static Arc<Mutex<LV2World>> {
    LV2_WORLD.get_or_init(|| Arc::new(Mutex::new(LV2World::new())))
}

struct LV2World {
    world: *mut LilvWorld,
    plugins: *const LilvPlugins,
}

impl LV2World {
    fn new() -> Self {
        unsafe {
            let world = lilv_world_new();
            if !world.is_null() {
                lilv_world_load_all(world);
            }
            let plugins = lilv_world_get_all_plugins(world);

            Self { world, plugins }
        }
    }

    fn get_world(&self) -> *mut LilvWorld {
        self.world
    }
    fn get_plugins(&self) -> *const LilvPlugins {
        self.plugins
    }
}

unsafe impl Send for LV2World {}

impl Drop for LV2World {
    fn drop(&mut self) {
        unsafe {
            if !self.world.is_null() {
                lilv_world_free(self.world);
                self.world = ptr::null_mut();
            }
        }
    }
}

// Internal port information
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub index: u32,
    pub symbol: String,
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub is_integer: bool,
    pub is_toggled: bool,
    pub is_enum: Option<HashMap<String, u32>>,

    // Whether this port is an input (host provides a value) or output (plugin writes)
    pub is_input: bool,
}

// URI constants
const LV2_AUDIO_PORT_URI: &[u8] = b"http://lv2plug.in/ns/lv2core#AudioPort\0";
const LV2_CONTROL_PORT_URI: &[u8] = b"http://lv2plug.in/ns/lv2core#ControlPort\0";
const LV2_INPUT_PORT_URI: &[u8] = b"http://lv2plug.in/ns/lv2core#InputPort\0";
const LV2_OUTPUT_PORT_URI: &[u8] = b"http://lv2plug.in/ns/lv2core#OutputPort\0";
const LV2_INTEGER_URI: &[u8] = b"http://lv2plug.in/ns/lv2core#integer\0";
const LV2_TOGGLED_URI: &[u8] = b"http://lv2plug.in/ns/lv2core#toggled\0";
const LV2_ENUM_URI: &[u8] = b"http://lv2plug.in/ns/lv2core#enumeration\0";

const LV2_SCALE_POINT_URI: &[u8] = b"http://lv2plug.in/ns/lv2core#scalePoint\0";
const LV2_RDF_LABEL: &[u8] = b"http://www.w3.org/2000/01/rdf-schema#label\0";
const LV2_RDF_VALUE: &[u8] = b"http://www.w3.org/1999/02/22-rdf-syntax-ns#value\0";

/// Get the human-readable name of an LV2 plugin by its URI
/// Returns None if the plugin is not found
pub fn get_plugin_name(plugin_uri: &str) -> Option<String> {
    unsafe {
        let world_guard = get_world().lock().ok()?;
        let world = world_guard.get_world();
        let plugins = world_guard.get_plugins();

        if world.is_null() {
            return None;
        }

        let uri_cstr = CString::new(plugin_uri).ok()?;
        let uri_node = lilv_new_uri(world, uri_cstr.as_ptr());

        if uri_node.is_null() {
            return None;
        }

        let plugin = lilv_plugins_get_by_uri(plugins, uri_node);

        let result = if !plugin.is_null() {
            let name_node = lilv_plugin_get_name(plugin);
            if !name_node.is_null() {
                let name = CStr::from_ptr(lilv_node_as_string(name_node))
                    .to_string_lossy()
                    .to_string();
                lilv_node_free(name_node);
                Some(name)
            } else {
                None
            }
        } else {
            None
        };

        lilv_node_free(uri_node);
        result
    }
}

/// A simple LV2 Plugin wrapper
pub struct LV2PluginBase {
    // LV2 objects provided by Lilv
    plugin: *const LilvPlugin,
    instance: *mut LilvInstance,
    uri_node: *mut LilvNode,

    // Port mappings
    audio_input_ports: Vec<u32>,
    audio_output_ports: Vec<u32>,
    pub control_ports: HashMap<String, PortInfo>,

    // Control port values
    // TODO: Can't remember why I made this, Revisit later.
    control_values: Vec<f32>,
    num_ports: u32,

    // Config
    pub plugin_uri: String,
    pub sample_rate: u32,
    pub max_block_size: usize,
}

impl LV2PluginBase {
    /// Create a new LV2 plugin instance
    pub fn new(plugin_uri: &str, rate: u32, max_block_size: usize) -> Result<Self, String> {
        unsafe {
            // Get shared world (initializes on first call)
            let (world, plugins) = {
                let world_guard = get_world()
                    .lock()
                    .map_err(|e| format!("Failed to lock world: {}", e))?;
                (world_guard.get_world(), world_guard.get_plugins())
            };

            if world.is_null() {
                return Err("Failed to access LV2 world".to_string());
            }

            let uri_cstr = CString::new(plugin_uri).map_err(|e| format!("Invalid URI: {}", e))?;
            let uri_node = lilv_new_uri(world, uri_cstr.as_ptr());

            if uri_node.is_null() {
                return Err("Failed to create URI node".to_string());
            }

            let plugin = lilv_plugins_get_by_uri(plugins, uri_node);
            if plugin.is_null() {
                lilv_node_free(uri_node);
                return Err(format!("Plugin not found: {}", plugin_uri));
            }

            // Create port class nodes
            let audio_class = lilv_new_uri(world, LV2_AUDIO_PORT_URI.as_ptr() as *const i8);
            let control_class = lilv_new_uri(world, LV2_CONTROL_PORT_URI.as_ptr() as *const i8);
            let input_class = lilv_new_uri(world, LV2_INPUT_PORT_URI.as_ptr() as *const i8);
            let output_class = lilv_new_uri(world, LV2_OUTPUT_PORT_URI.as_ptr() as *const i8);
            let integer_class = lilv_new_uri(world, LV2_INTEGER_URI.as_ptr() as *const i8);
            let toggled_class = lilv_new_uri(world, LV2_TOGGLED_URI.as_ptr() as *const i8);

            // Enum Stuff
            let enum_class = lilv_new_uri(world, LV2_ENUM_URI.as_ptr() as *const i8);
            let scale_point_class = lilv_new_uri(world, LV2_SCALE_POINT_URI.as_ptr() as *const i8);
            let rdf_label = lilv_new_uri(world, LV2_RDF_LABEL.as_ptr() as *const i8);
            let rdf_value = lilv_new_uri(world, LV2_RDF_VALUE.as_ptr() as *const i8);

            // Scan ports
            let num_ports = lilv_plugin_get_num_ports(plugin);
            let mut audio_input_ports = Vec::new();
            let mut audio_output_ports = Vec::new();
            let mut control_ports = HashMap::new();

            // Pre-allocate control values vec for stable pointers (elements never move)
            let mut control_values = vec![0.0f32; num_ports as usize];

            for port_idx in 0..num_ports {
                let port = lilv_plugin_get_port_by_index(plugin, port_idx);
                if port.is_null() {
                    continue;
                }

                let is_audio = lilv_port_is_a(plugin, port, audio_class);
                let is_control = lilv_port_is_a(plugin, port, control_class);
                let is_input = lilv_port_is_a(plugin, port, input_class);
                let is_output = lilv_port_is_a(plugin, port, output_class);

                if is_audio {
                    if is_input {
                        audio_input_ports.push(port_idx);
                    } else if is_output {
                        audio_output_ports.push(port_idx);
                    }
                } else if is_control {
                    let symbol_node = lilv_port_get_symbol(plugin, port);
                    let symbol = if !symbol_node.is_null() {
                        let s = CStr::from_ptr(lilv_node_as_string(symbol_node))
                            .to_string_lossy()
                            .to_string();

                        // Free the symbol
                        lilv_node_free(symbol_node as *mut LilvNode);
                        s
                    } else {
                        format!("control_{}", port_idx)
                    };

                    let name_node = lilv_port_get_name(plugin, port);
                    let name = if !name_node.is_null() {
                        let n = CStr::from_ptr(lilv_node_as_string(name_node))
                            .to_string_lossy()
                            .to_string();
                        lilv_node_free(name_node as *mut LilvNode);
                        n
                    } else {
                        symbol.clone()
                    };

                    let mut def_node: *mut LilvNode = ptr::null_mut();
                    let mut min_node: *mut LilvNode = ptr::null_mut();
                    let mut max_node: *mut LilvNode = ptr::null_mut();

                    lilv_port_get_range(plugin, port, &mut def_node, &mut min_node, &mut max_node);

                    let default = if !def_node.is_null() {
                        let val = lilv_node_as_float(def_node);
                        lilv_node_free(def_node);
                        val
                    } else {
                        0.0
                    };

                    let min = if !min_node.is_null() {
                        let val = lilv_node_as_float(min_node);
                        lilv_node_free(min_node);
                        val
                    } else {
                        0.0
                    };

                    let max = if !max_node.is_null() {
                        let val = lilv_node_as_float(max_node);
                        lilv_node_free(max_node);
                        val
                    } else {
                        1.0
                    };

                    let is_integer = lilv_port_has_property(plugin, port, integer_class);
                    let is_toggled = lilv_port_has_property(plugin, port, toggled_class);
                    let is_enum = lilv_port_has_property(plugin, port, enum_class);

                    let is_enum = if is_enum {
                        let mut result = HashMap::new();
                        let scale_points = lilv_port_get_value(plugin, port, scale_point_class);

                        let mut iter = lilv_nodes_begin(scale_points);
                        while !lilv_nodes_is_end(scale_points, iter) {
                            let scale_point = lilv_nodes_get(scale_points, iter);

                            // Get the label (human-readable name)
                            let labels =
                                lilv_world_find_nodes(world, scale_point, rdf_label, ptr::null());

                            if lilv_nodes_size(labels) > 0 {
                                let label_node = lilv_nodes_get_first(labels);
                                let label_cstr = lilv_node_as_string(label_node);
                                let label = CStr::from_ptr(label_cstr).to_string_lossy();

                                // Get the value (numeric value)
                                let values = lilv_world_find_nodes(
                                    world,
                                    scale_point,
                                    rdf_value,
                                    ptr::null(),
                                );

                                if lilv_nodes_size(values) > 0 {
                                    let value_node = lilv_nodes_get_first(values);
                                    let value = lilv_node_as_int(value_node); // or lilv_node_as_int

                                    result.insert(label.to_string(), value as u32);
                                }

                                lilv_nodes_free(values);
                            }

                            lilv_nodes_free(labels);
                            iter = lilv_nodes_next(scale_points, iter);
                        }

                        lilv_nodes_free(scale_points);
                        Some(result)
                    } else {
                        None
                    };

                    let port_info = PortInfo {
                        index: port_idx,
                        symbol: symbol.clone(),
                        name,
                        min,
                        max,
                        default,
                        is_integer,
                        is_toggled,
                        is_enum,
                        is_input,
                    };

                    debug!("LV2 Control Port Found: {:?}", port_info);

                    control_values[port_idx as usize] = default;
                    control_ports.insert(symbol, port_info);
                }
            }

            lilv_node_free(audio_class);
            lilv_node_free(control_class);
            lilv_node_free(input_class);
            lilv_node_free(output_class);
            lilv_node_free(integer_class);
            lilv_node_free(toggled_class);
            lilv_node_free(enum_class);
            lilv_node_free(scale_point_class);
            lilv_node_free(rdf_label);
            lilv_node_free(rdf_value);

            let instance = lilv_plugin_instantiate(plugin, rate as f64, ptr::null());
            if instance.is_null() {
                lilv_node_free(uri_node);
                return Err("Failed to instantiate plugin".to_string());
            }

            let mut plugin_base = Self {
                plugin,
                instance,
                uri_node,
                audio_input_ports,
                audio_output_ports,
                control_ports,
                control_values,
                num_ports,

                plugin_uri: plugin_uri.to_string(),
                sample_rate: rate,
                max_block_size,
            };

            // Connect control ports once during initialization
            plugin_base.connect_control_ports();

            lilv_instance_activate(instance);

            Ok(plugin_base)
        }
    }

    /// Connect control ports to their buffers (called once during initialization)
    /// Audio ports are connected per-process call for zero-copy operation
    fn connect_control_ports(&mut self) {
        unsafe {
            // Connect control ports to stable Vec addresses. Only connect control inputs;
            // control outputs are written by the plugin and should not be connected to host storage.
            for port in self.control_ports.values() {
                if port.is_input {
                    let value_ptr = &self.control_values[port.index as usize] as *const f32;
                    lilv_instance_connect_port(self.instance, port.index, value_ptr as *mut c_void);
                }
            }
        }
    }

    /// Connect control output ports to host-side buffers so the host can read plugin-written values.
    /// This is optional and should be used when you want to observe plugin output controls.
    pub fn connect_control_outputs(&mut self) {
        unsafe {
            for port in self.control_ports.values() {
                if !port.is_input {
                    let value_ptr = &self.control_values[port.index as usize] as *const f32;
                    lilv_instance_connect_port(self.instance, port.index, value_ptr as *mut c_void);
                }
            }
        }
    }

    /// Read the current values of control output ports.
    /// Returns a HashMap mapping symbol -> value. Only includes ports marked as outputs.
    pub fn read_control_outputs(&self) -> HashMap<String, f32> {
        let mut map = HashMap::new();
        for (sym, port) in &self.control_ports {
            if !port.is_input {
                // safe to read from control_values since it's host-owned storage
                if let Some(v) = self.get_control_by_symbol(sym) {
                    map.insert(sym.clone(), v);
                }
            }
        }
        map
    }

    /// Disconnect control output ports by setting their instance pointer to null.
    /// This is defensive and ensures the plugin can't write into freed host memory if we drop buffers.
    pub fn disconnect_control_outputs(&mut self) {
        unsafe {
            for port in self.control_ports.values() {
                if !port.is_input {
                    lilv_instance_connect_port(self.instance, port.index, ptr::null_mut());
                }
            }
        }
    }

    /// Get control port value by symbol
    pub fn get_control_by_symbol(&self, symbol: &str) -> Option<f32> {
        self.control_ports
            .get(symbol)
            .map(|port| self.control_values[port.index as usize])
    }

    /// Trait-based typed getter/setter support for control ports.
    /// Implementations convert between Rust types and f32 used by LV2 control_values.
    pub fn get_control_typed<T: ControlConvert + Debug>(&self, symbol: &str) -> Option<T> {
        self.get_control_by_symbol(symbol).map(|v| T::from_f32(v))
    }

    /// Set a control port using a typed value. Performs rounding for integer ports and clamps
    /// to the port min/max. Returns Err if the symbol is unknown.
    pub fn set_control_typed<T: ControlConvert + Debug>(
        &mut self,
        symbol: &str,
        value: T,
    ) -> Result<(), String> {
        if let Some(port) = self.control_ports.get(symbol) {
            let mut vf = value.to_f32();
            if port.is_integer {
                vf = vf.round();
            }
            vf = vf.max(port.min).min(port.max);
            self.control_values[port.index as usize] = vf;
            Ok(())
        } else {
            Err(format!("Unknown control port: {}", symbol))
        }
    }

    /// Set a control port using a typed value with warning logging and callback.
    pub fn set_typed_prop<T, F>(&mut self, symbol: &str, value: T, update: F)
    where
        T: ControlConvert + Debug + Clone,
        F: FnOnce(T),
    {
        if let Err(e) = self.set_control_typed(symbol, value.clone()) {
            warn!("Failed to set '{}' -> {:?}: {}", symbol, value, e);
        } else {
            update(value);
        }
    }

    /// Process audio samples
    #[inline]
    pub fn process(&mut self, mut inputs: Vec<&mut [f32]>, mut outputs: Vec<&mut [f32]>) {
        // Validate port counts
        if inputs.len() != self.audio_input_ports.len()
            || outputs.len() != self.audio_output_ports.len()
        {
            return;
        }

        // Find the minimum buffer size
        let sample_count = inputs
            .iter()
            .map(|buf| buf.len())
            .chain(outputs.iter().map(|buf| buf.len()))
            .min()
            .unwrap_or(0);

        // Validate sample count
        if sample_count == 0 || sample_count > self.max_block_size {
            return;
        }

        unsafe {
            // This should be a zero-copy operation, we just attach the plugin port directly
            // to the PipeWire provided buffers.
            for (i, &port_idx) in self.audio_input_ports.iter().enumerate() {
                lilv_instance_connect_port(
                    self.instance,
                    port_idx,
                    inputs[i].as_mut_ptr() as *mut c_void,
                );
            }

            for (i, &port_idx) in self.audio_output_ports.iter().enumerate() {
                lilv_instance_connect_port(
                    self.instance,
                    port_idx,
                    outputs[i].as_mut_ptr() as *mut c_void,
                );
            }

            lilv_instance_run(self.instance, sample_count as u32);
        }
    }

    pub fn num_inputs(&self) -> usize {
        self.audio_input_ports.len()
    }

    pub fn num_outputs(&self) -> usize {
        self.audio_output_ports.len()
    }
}

unsafe impl Send for LV2PluginBase {}

impl Drop for LV2PluginBase {
    fn drop(&mut self) {
        unsafe {
            if !self.instance.is_null() {
                lilv_instance_deactivate(self.instance);
                lilv_instance_free(self.instance);
            }
            if !self.uri_node.is_null() {
                lilv_node_free(self.uri_node);
            }
            // World is shared and managed by OnceLock
        }
    }
}

/// Trait to convert between Rust types and the f32 representation used in LV2 control ports.
pub trait ControlConvert: Sized {
    fn to_f32(self) -> f32;
    fn from_f32(v: f32) -> Self;
}

impl ControlConvert for f32 {
    fn to_f32(self) -> f32 {
        self
    }
    fn from_f32(v: f32) -> Self {
        v
    }
}

impl ControlConvert for i32 {
    fn to_f32(self) -> f32 {
        self as f32
    }
    fn from_f32(v: f32) -> Self {
        v as i32
    }
}

impl ControlConvert for bool {
    fn to_f32(self) -> f32 {
        if self { 1.0 } else { 0.0 }
    }
    fn from_f32(v: f32) -> Self {
        v > 0.5
    }
}
