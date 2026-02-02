use lilv_sys::{
    LilvInstance, LilvNode, LilvPlugin, LilvPlugins, LilvWorld, lilv_instance_activate,
    lilv_instance_connect_port, lilv_instance_deactivate, lilv_instance_free, lilv_instance_run,
    lilv_new_uri, lilv_node_as_float, lilv_node_as_int, lilv_node_as_string, lilv_node_free,
    lilv_nodes_begin, lilv_nodes_free, lilv_nodes_get, lilv_nodes_get_first, lilv_nodes_is_end,
    lilv_nodes_next, lilv_nodes_size, lilv_plugin_get_name, lilv_plugin_get_num_ports,
    lilv_plugin_get_num_ports_of_class, lilv_plugin_get_port_by_index, lilv_plugin_get_uri,
    lilv_plugin_instantiate, lilv_plugins_begin, lilv_plugins_get, lilv_plugins_get_by_uri,
    lilv_plugins_is_end, lilv_plugins_next, lilv_port_get_name, lilv_port_get_range,
    lilv_port_get_symbol, lilv_port_get_value, lilv_port_has_property, lilv_port_is_a,
    lilv_world_find_nodes, lilv_world_free, lilv_world_get_all_plugins, lilv_world_load_all,
    lilv_world_new,
};
use log::{debug, warn};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fmt::Debug;
use std::os::raw::c_void;
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};

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
const LV2_RDF_TYPE: &[u8] = b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type\0";

// Port Groups extension URIs
const LV2_PORT_GROUP_URI: &[u8] = b"http://lv2plug.in/ns/ext/port-groups#group\0";
const LV2_STEREO_GROUP_URI: &[u8] = b"http://lv2plug.in/ns/ext/port-groups#StereoGroup\0";
const LV2_MONO_GROUP_URI: &[u8] = b"http://lv2plug.in/ns/ext/port-groups#MonoGroup\0";
const LV2_SIDECHAIN_GROUP_URI: &[u8] = b"http://lv2plug.in/ns/ext/port-groups#SideChainGroup\0";

// String slice versions of URIs for comparison (without null terminator)
const LV2_STEREO_GROUP_URI_STR: &str = "http://lv2plug.in/ns/ext/port-groups#StereoGroup";
const LV2_SIDECHAIN_GROUP_URI_STR: &str = "http://lv2plug.in/ns/ext/port-groups#SideChainGroup";

// Shared LV2 world (initialized once, shared by all plugins)
static LV2_WORLD: OnceLock<Arc<Mutex<LV2World>>> = OnceLock::new();
fn get_world() -> &'static Arc<Mutex<LV2World>> {
    LV2_WORLD.get_or_init(|| Arc::new(Mutex::new(LV2World::new())))
}

struct LV2World {
    world: *mut LilvWorld,
    plugins: *const LilvPlugins,

    /// A full list of all LV2 plugins by URI
    plugin_list: Vec<String>,

    /// Pre-allocated URI nodes, in a previous revision I was generating these on-the-fly and
    /// dropping them when I was done with them, but there's no reason to do that, the nodes can
    /// be kept cached for the duration of the World.
    uri_nodes: UriNodes,
}

/// Pre-allocated URI nodes for common LV2 predicates and classes
/// These are created once and reused throughout the lifetime of LV2World
struct UriNodes {
    // Core port types
    audio_port: *mut LilvNode,
    control_port: *mut LilvNode,
    input_port: *mut LilvNode,
    output_port: *mut LilvNode,

    // Port properties
    integer: *mut LilvNode,
    toggled: *mut LilvNode,
    enumeration: *mut LilvNode,

    // Scale points
    scale_point: *mut LilvNode,

    // RDF predicates
    rdf_type: *mut LilvNode,
    rdf_label: *mut LilvNode,
    rdf_value: *mut LilvNode,

    // Port groups
    port_group: *mut LilvNode,
    stereo_group: *mut LilvNode,
    mono_group: *mut LilvNode,
    sidechain_group: *mut LilvNode,
}

impl UriNodes {
    unsafe fn new(world: *mut LilvWorld) -> Self {
        // SAFETY: All lilv_new_uri calls are within unsafe block as required by Rust 2024
        Self {
            audio_port: unsafe { lilv_new_uri(world, LV2_AUDIO_PORT_URI.as_ptr() as *const i8) },
            control_port: unsafe {
                lilv_new_uri(world, LV2_CONTROL_PORT_URI.as_ptr() as *const i8)
            },
            input_port: unsafe { lilv_new_uri(world, LV2_INPUT_PORT_URI.as_ptr() as *const i8) },
            output_port: unsafe { lilv_new_uri(world, LV2_OUTPUT_PORT_URI.as_ptr() as *const i8) },
            integer: unsafe { lilv_new_uri(world, LV2_INTEGER_URI.as_ptr() as *const i8) },
            toggled: unsafe { lilv_new_uri(world, LV2_TOGGLED_URI.as_ptr() as *const i8) },
            enumeration: unsafe { lilv_new_uri(world, LV2_ENUM_URI.as_ptr() as *const i8) },
            scale_point: unsafe { lilv_new_uri(world, LV2_SCALE_POINT_URI.as_ptr() as *const i8) },
            rdf_type: unsafe { lilv_new_uri(world, LV2_RDF_TYPE.as_ptr() as *const i8) },
            rdf_label: unsafe { lilv_new_uri(world, LV2_RDF_LABEL.as_ptr() as *const i8) },
            rdf_value: unsafe { lilv_new_uri(world, LV2_RDF_VALUE.as_ptr() as *const i8) },
            port_group: unsafe { lilv_new_uri(world, LV2_PORT_GROUP_URI.as_ptr() as *const i8) },
            stereo_group: unsafe {
                lilv_new_uri(world, LV2_STEREO_GROUP_URI.as_ptr() as *const i8)
            },
            mono_group: unsafe { lilv_new_uri(world, LV2_MONO_GROUP_URI.as_ptr() as *const i8) },
            sidechain_group: unsafe {
                lilv_new_uri(world, LV2_SIDECHAIN_GROUP_URI.as_ptr() as *const i8)
            },
        }
    }

    unsafe fn free(&mut self) {
        // SAFETY: All lilv_node_free calls are within unsafe blocks as required by Rust 2024
        unsafe {
            lilv_node_free(self.audio_port);
            lilv_node_free(self.control_port);
            lilv_node_free(self.input_port);
            lilv_node_free(self.output_port);
            lilv_node_free(self.integer);
            lilv_node_free(self.toggled);
            lilv_node_free(self.enumeration);
            lilv_node_free(self.scale_point);
            lilv_node_free(self.rdf_type);
            lilv_node_free(self.rdf_label);
            lilv_node_free(self.rdf_value);
            lilv_node_free(self.port_group);
            lilv_node_free(self.stereo_group);
            lilv_node_free(self.mono_group);
            lilv_node_free(self.sidechain_group);
        }
    }
}

impl LV2World {
    fn new() -> Self {
        unsafe {
            let world = lilv_world_new();
            if !world.is_null() {
                lilv_world_load_all(world);
            }
            let plugins = lilv_world_get_all_plugins(world);

            // Pre-allocate all URI nodes once
            let uri_nodes = UriNodes::new(world);

            // Get the Full Plugin List
            let plugin_list = Self::get_plugin_list(plugins);

            Self {
                world,
                plugins,
                plugin_list,
                uri_nodes,
            }
        }
    }

    pub fn get_plugin_list(plugins: *const LilvPlugins) -> Vec<String> {
        let mut plugin_uris: Vec<String> = Vec::new();
        unsafe {
            let mut iter = lilv_plugins_begin(plugins);
            while !lilv_plugins_is_end(plugins, iter) {
                let plugin = lilv_plugins_get(plugins, iter);
                if !plugin.is_null() {
                    let uri_node = lilv_plugin_get_uri(plugin);
                    if !uri_node.is_null() {
                        let uri_cstr = lilv_node_as_string(uri_node);
                        let uri = CStr::from_ptr(uri_cstr).to_string_lossy().to_string();
                        plugin_uris.push(uri);
                    }
                }
                iter = lilv_plugins_next(plugins, iter);
            }
        }
        plugin_uris
    }

    pub fn validate_plugin(&self, uri: &str) -> bool {
        // Fast Fail, check if plugin URI is in the known list
        if !self.plugin_list.contains(&uri.to_string()) {
            return false;
        }

        unsafe {
            let uri_cstr = match CString::new(uri) {
                Ok(s) => s,
                Err(_) => return false,
            };

            let uri_node = lilv_new_uri(self.world, uri_cstr.as_ptr());
            if uri_node.is_null() {
                return false;
            }

            let plugin = lilv_plugins_get_by_uri(self.plugins, uri_node);
            lilv_node_free(uri_node);

            if plugin.is_null() {
                return false;
            }

            // Get plugin name for logging
            let plugin_name = {
                let name_node = lilv_plugin_get_name(plugin);
                if !name_node.is_null() {
                    let name_cstr = lilv_node_as_string(name_node);
                    CStr::from_ptr(name_cstr).to_string_lossy().to_string()
                } else {
                    "Unknown".to_string()
                }
            };

            // Use lilv_plugin_get_num_ports_of_class for quick check
            let audio_input_count = lilv_plugin_get_num_ports_of_class(
                plugin,
                self.uri_nodes.audio_port,
                self.uri_nodes.input_port,
                ptr::null::<LilvNode>(),
            );
            let audio_output_count = lilv_plugin_get_num_ports_of_class(
                plugin,
                self.uri_nodes.audio_port,
                self.uri_nodes.output_port,
                ptr::null::<LilvNode>(),
            );

            // We're going to try to fast fail here, if neither the input or outputs are stereo,
            // we can reject the plugin outright without further analysis.
            //
            // Note: We need to consider that some plugins may have sidechain inputs/outputs, which
            // will be reviewed after. So 3 or 4 channel inputs are acceptable at this stage.
            if !(2..=4).contains(&audio_input_count) || audio_output_count != 2 {
                let reason = if audio_input_count == 0 {
                    "no audio inputs (generator/instrument)"
                } else if audio_input_count == 1 {
                    "mono input (requires stereo)"
                } else if audio_input_count > 4 {
                    "multi-channel input (>4 inputs)"
                } else if audio_output_count == 0 {
                    "no audio outputs (analyzer)"
                } else if audio_output_count == 1 {
                    "mono output (requires stereo)"
                } else if audio_output_count > 2 {
                    "multi-channel output (>2 outputs)"
                } else {
                    "unknown reason"
                };

                debug!(
                    "LV2 Plugin REJECTED: '{}' ({}i/{}o) - {}",
                    plugin_name, audio_input_count, audio_output_count, reason
                );
                return false;
            }

            // Now do detailed analysis with sidechain detection
            let num_ports = lilv_plugin_get_num_ports(plugin);
            let mut main_input_count = 0;
            let mut main_output_count = 0;

            for port_idx in 0..num_ports {
                let port = lilv_plugin_get_port_by_index(plugin, port_idx);
                if !port.is_null() {
                    let is_audio = lilv_port_is_a(plugin, port, self.uri_nodes.audio_port);
                    let is_input = lilv_port_is_a(plugin, port, self.uri_nodes.input_port);
                    let is_output = lilv_port_is_a(plugin, port, self.uri_nodes.output_port);

                    if is_audio {
                        let group_nodes =
                            lilv_port_get_value(plugin, port, self.uri_nodes.port_group);

                        let is_sidechain = if lilv_nodes_size(group_nodes) > 0 {
                            let group_node = lilv_nodes_get_first(group_nodes);
                            let uri_cstr = lilv_node_as_string(group_node);
                            let group_uri = CStr::from_ptr(uri_cstr);

                            // Query RDF to check if it's a sidechain group
                            let group_uri_node = lilv_new_uri(self.world, group_uri.as_ptr());
                            let mut is_sc = false;

                            if !group_uri_node.is_null() {
                                let type_nodes = lilv_world_find_nodes(
                                    self.world,
                                    group_uri_node,
                                    self.uri_nodes.rdf_type,
                                    ptr::null(),
                                );

                                let mut iter = lilv_nodes_begin(type_nodes);
                                while !lilv_nodes_is_end(type_nodes, iter) {
                                    let type_node = lilv_nodes_get(type_nodes, iter);
                                    let type_uri_cstr = lilv_node_as_string(type_node);
                                    let type_uri = CStr::from_ptr(type_uri_cstr).to_string_lossy();

                                    if type_uri == LV2_SIDECHAIN_GROUP_URI_STR {
                                        is_sc = true;
                                        break;
                                    }

                                    iter = lilv_nodes_next(type_nodes, iter);
                                }
                                lilv_nodes_free(type_nodes);
                                lilv_node_free(group_uri_node);
                            }

                            is_sc
                        } else {
                            false
                        };

                        lilv_nodes_free(group_nodes);

                        // Count main (non-sidechain) ports
                        if !is_sidechain {
                            if is_input {
                                main_input_count += 1;
                                if main_input_count > 2 {
                                    debug!(
                                        "LV2 Plugin REJECTED: '{}' ({}i/{}o) - multi-channel input (>2 inputs)",
                                        plugin_name, main_input_count, main_output_count
                                    );
                                    return false;
                                }
                            } else if is_output {
                                main_output_count += 1;
                                if main_output_count > 2 {
                                    debug!(
                                        "LV2 Plugin REJECTED: '{}' ({}i/{}o) - multi-channel output (>2 outputs)",
                                        plugin_name, main_input_count, main_output_count
                                    );
                                    return false;
                                }
                            }
                        }
                    }
                }
            }

            // Accept if exactly 2 main inputs and 2 main outputs
            if main_input_count == 2 && main_output_count == 2 {
                debug!(
                    "LV2 Plugin ACCEPTED: '{}' ({}i/{}o) -> {}",
                    plugin_name, main_input_count, main_output_count, uri
                );
                true
            } else {
                let reason = if main_input_count == 0 {
                    "no main audio inputs"
                } else if main_input_count == 1 {
                    "mono input (requires stereo)"
                } else if main_output_count == 0 {
                    "no main audio outputs"
                } else if main_output_count == 1 {
                    "mono output (requires stereo)"
                } else {
                    "incorrect main port configuration"
                };

                debug!(
                    "LV2 Plugin REJECTED: '{}' ({}i/{}o) - {}",
                    plugin_name, main_input_count, main_output_count, reason
                );
                false
            }
        }
    }

    fn get_world(&self) -> *mut LilvWorld {
        self.world
    }
    fn get_plugins(&self) -> *const LilvPlugins {
        self.plugins
    }
}

// SAFETY: LV2World is Send because we throw the current usage off to Pipewire which will
// make all the calls on its own thread. LilvWorld and related objects are not thread-safe.
unsafe impl Send for LV2World {}

impl Drop for LV2World {
    fn drop(&mut self) {
        unsafe {
            // Free URI nodes first (they depend on world being valid)
            self.uri_nodes.free();

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
    pub is_enum: Option<HashMap<u32, String>>,

    // Whether this port is an input (host provides a value) or output (plugin writes)
    pub is_input: bool,
}

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
            // NOTE: lilv_plugin_get_name() returns a plugin-owned node - DO NOT FREE IT
            let name_node = lilv_plugin_get_name(plugin);
            if !name_node.is_null() {
                Some(
                    CStr::from_ptr(lilv_node_as_string(name_node))
                        .to_string_lossy()
                        .to_string(),
                )
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

    // Control ports indexed by port_index (may have gaps for audio ports)
    pub control_ports: Vec<Option<PortInfo>>,

    // Symbol to port index mapping (public for reverse lookups)
    pub symbol_to_port: HashMap<String, u32>,

    // Control port values, these point directly to the LV2 internal values
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
            let world_guard = get_world()
                .lock()
                .map_err(|e| format!("Failed to lock world: {}", e))?;

            let world = world_guard.get_world();
            let plugins = world_guard.get_plugins();
            let uri_nodes = &world_guard.uri_nodes;

            if world.is_null() {
                return Err("Failed to access LV2 world".to_string());
            }

            if !world_guard.validate_plugin(plugin_uri) {
                return Err(format!("Plugin '{}' failed validation", plugin_uri));
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

            // Scan ports
            let num_ports = lilv_plugin_get_num_ports(plugin);
            let mut audio_input_ports = Vec::new();
            let mut audio_output_ports = Vec::new();

            // Pre-allocate control ports Vec with None entries (indexed by port_index)
            let mut control_ports: Vec<Option<PortInfo>> = vec![None; num_ports as usize];
            let mut symbol_to_port = HashMap::new();

            // Pre-allocate control values vec for stable pointers (elements never move)
            let mut control_values = vec![0.0f32; num_ports as usize];

            for port_idx in 0..num_ports {
                let port = lilv_plugin_get_port_by_index(plugin, port_idx);
                if port.is_null() {
                    continue;
                }

                let is_audio = lilv_port_is_a(plugin, port, uri_nodes.audio_port);
                let is_control = lilv_port_is_a(plugin, port, uri_nodes.control_port);
                let is_input = lilv_port_is_a(plugin, port, uri_nodes.input_port);
                let is_output = lilv_port_is_a(plugin, port, uri_nodes.output_port);

                if is_audio {
                    if is_input {
                        audio_input_ports.push(port_idx);
                    } else if is_output {
                        audio_output_ports.push(port_idx);
                    }
                } else if is_control {
                    // Reference Note: DONT FREE THIS!
                    let symbol_node = lilv_port_get_symbol(plugin, port);
                    let symbol = if !symbol_node.is_null() {
                        CStr::from_ptr(lilv_node_as_string(symbol_node))
                            .to_string_lossy()
                            .to_string()
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

                    let is_integer = lilv_port_has_property(plugin, port, uri_nodes.integer);
                    let is_toggled = lilv_port_has_property(plugin, port, uri_nodes.toggled);
                    let is_enum = lilv_port_has_property(plugin, port, uri_nodes.enumeration);

                    let is_enum = if is_enum {
                        let mut result = HashMap::new();
                        let scale_points = lilv_port_get_value(plugin, port, uri_nodes.scale_point);

                        let mut iter = lilv_nodes_begin(scale_points);
                        while !lilv_nodes_is_end(scale_points, iter) {
                            let scale_point = lilv_nodes_get(scale_points, iter);

                            // Get the label (human-readable name)
                            let labels = lilv_world_find_nodes(
                                world,
                                scale_point,
                                uri_nodes.rdf_label,
                                ptr::null(),
                            );

                            if lilv_nodes_size(labels) > 0 {
                                let label_node = lilv_nodes_get_first(labels);
                                let label_cstr = lilv_node_as_string(label_node);
                                let label = CStr::from_ptr(label_cstr).to_string_lossy();

                                // Get the value (numeric value)
                                let values = lilv_world_find_nodes(
                                    world,
                                    scale_point,
                                    uri_nodes.rdf_value,
                                    ptr::null(),
                                );

                                if lilv_nodes_size(values) > 0 {
                                    let value_node = lilv_nodes_get_first(values);
                                    let value = lilv_node_as_int(value_node); // or lilv_node_as_int

                                    result.insert(value as u32, label.to_string());
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
                    control_ports[port_idx as usize] = Some(port_info);
                    symbol_to_port.insert(symbol, port_idx);
                }
            }

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
                symbol_to_port,
                control_values,
                num_ports,

                plugin_uri: plugin_uri.to_string(),
                sample_rate: rate,
                max_block_size,
            };

            // Connect all control ports once during initialization (both inputs and outputs)
            plugin_base.connect_control_ports();

            lilv_instance_activate(instance);

            Ok(plugin_base)
        }
    }

    /// Connect all control ports to their buffers (called once during initialization)
    fn connect_control_ports(&mut self) {
        unsafe {
            for port in self.control_ports.iter().flatten() {
                let value_ptr = &self.control_values[port.index as usize] as *const f32;
                lilv_instance_connect_port(self.instance, port.index, value_ptr as *mut c_void);
            }
        }
    }

    /// Read the current values of control output ports.
    /// TODO: Possibly allow individual lookups? For now this is fine.
    pub fn read_control_outputs(&self) -> HashMap<String, f32> {
        let mut map = HashMap::new();
        for port_opt in &self.control_ports {
            if let Some(port) = port_opt
                && !port.is_input
                && let Some(v) = self.get_control_by_index(port.index)
            {
                map.insert(port.symbol.clone(), v);
            }
        }
        map
    }

    /// Get control port value by symbol
    pub fn get_control_by_symbol(&self, symbol: &str) -> Option<f32> {
        self.symbol_to_port
            .get(symbol)
            .and_then(|&port_idx| self.get_control_by_index(port_idx))
    }

    /// Get control port value by direct port index (faster than symbol lookup)
    #[inline]
    pub fn get_control_by_index(&self, port_index: u32) -> Option<f32> {
        let idx = port_index as usize;
        if idx < self.control_values.len() {
            Some(self.control_values[idx])
        } else {
            None
        }
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
        if let Some(&port_idx) = self.symbol_to_port.get(symbol) {
            if let Some(port) = &self.control_ports[port_idx as usize] {
                let mut vf = value.to_f32();
                if port.is_integer {
                    vf = vf.round();
                }
                vf = vf.max(port.min).min(port.max);
                self.control_values[port_idx as usize] = vf;
                Ok(())
            } else {
                Err(format!("Port index {} has no port info", port_idx))
            }
        } else {
            Err(format!("Unknown control port: {}", symbol))
        }
    }

    /// Set a control port by direct port index with type conversion and validation
    /// Looks up port metadata internally for validation
    #[inline]
    pub fn set_control_by_index_typed<T: ControlConvert + Debug>(&mut self, id: u32, value: T) {
        let idx = id as usize;
        if idx >= self.control_ports.len() {
            return;
        }

        // Set the value (with rounding/clamping as needed) directly into our control_values
        if let Some(port) = &self.control_ports[idx] {
            let mut vf = value.to_f32();
            if port.is_integer {
                vf = vf.round();
            }
            vf = vf.max(port.min).min(port.max);
            self.control_values[idx] = vf;
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
