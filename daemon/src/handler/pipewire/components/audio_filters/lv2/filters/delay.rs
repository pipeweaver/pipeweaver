use crate::handler::pipewire::components::audio_filters::lv2::base::LV2PluginBase;
use crate::{APP_ID, APP_NAME, APP_NAME_ID};
use log::warn;
use pipeweaver_pipewire::{
    FilterHandler, FilterProperties, FilterProperty, FilterValue, MediaClass,
};
use std::collections::HashMap;
use ulid::Ulid;

// Plugin URI for LSP comp_delay_x2_stereo
const LSP_COMP_DELAY_URI: &str = "http://lsp-plug.in/plugins/lv2/comp_delay_x2_stereo";

// Canonical short keys for the plugin's control ports
const PORT_ENABLED: &str = "enabled";
const PORT_MODE_LEFT: &str = "mode_l";
const PORT_MODE_RIGHT: &str = "mode_r";
const PORT_TIME_LEFT: &str = "time_l";
const PORT_TIME_RIGHT: &str = "time_r";

// Pipeweaver Property IDs (passed to the frontend)
const PROP_ENABLED: u32 = 0;
const PROP_MODE_LEFT: u32 = 1;
const PROP_MODE_RIGHT: u32 = 2;
const PROP_TIME_LEFT: u32 = 3;
const PROP_TIME_RIGHT: u32 = 4;

// A macro to reduce boilerplate when matching property IDs and value variants
macro_rules! prop_if_chains {
    ($id_ident:ident, $self_ident:ident, $vref:ident, $( ($pid:ident, $variant:path, $key:expr, $ty:ty, $field:ident) ),* $(,)? ) => {
        $(
            if $id_ident == $pid {
                if let &$variant(__val) = $vref {
                    $self_ident.plugin.set_typed_prop::<$ty, _>($key, __val, |v| $self_ident.$field = v);
                    return;
                }
            }
        )*
    };
}

pub struct DelayFilter {
    plugin: LV2PluginBase,

    // Filter specific properties
    enabled: bool,
    mode_left: i32,
    mode_right: i32,
    time_left: f32,
    time_right: f32,
}

impl FilterHandler for DelayFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![
            self.get_property(PROP_ENABLED),
            self.get_property(PROP_MODE_LEFT),
            self.get_property(PROP_MODE_RIGHT),
            self.get_property(PROP_TIME_LEFT),
            self.get_property(PROP_TIME_RIGHT),
        ]
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        // Future Me Problem, I should be able to populate Min / Max / Enum from the LV2 plugin metadata

        match id {
            PROP_ENABLED => FilterProperty {
                id,
                name: "Enabled".into(),
                symbol: "enabled".into(),
                value: FilterValue::Bool(self.enabled),
                min: 0.0,
                max: 0.0,
                enum_def: None,
            },
            PROP_MODE_LEFT => FilterProperty {
                id,
                name: "Mode Left".into(),
                symbol: "mode_l".into(),
                value: FilterValue::Int32(self.mode_left),
                min: 0.0,
                max: 2.0,
                enum_def: Some(HashMap::from([
                    ("Samples".into(), 0),
                    ("Distance".into(), 1),
                    ("Time".into(), 2),
                ])),
            },
            PROP_MODE_RIGHT => FilterProperty {
                id,
                name: "Mode Right".into(),
                symbol: "mode_r".into(),
                value: FilterValue::Int32(self.mode_right),
                min: 0.0,
                max: 2.0,
                enum_def: Some(HashMap::from([
                    ("Samples".into(), 0),
                    ("Distance".into(), 1),
                    ("Time".into(), 2),
                ])),
            },
            PROP_TIME_LEFT => FilterProperty {
                id,
                name: "Time Left".into(),
                symbol: "time_l".into(),
                value: FilterValue::Float32(self.time_left),
                min: 0.0,
                max: 1000.0,
                enum_def: None,
            },
            PROP_TIME_RIGHT => FilterProperty {
                id,
                name: "Time Right".into(),
                symbol: "time_r".into(),
                value: FilterValue::Float32(self.time_right),
                min: 0.0,
                max: 1000.0,
                enum_def: None,
            },
            _ => panic!("Attempted to get non-existent property"),
        }
    }

    #[rustfmt::skip]
    fn set_property(&mut self, id: u32, value: FilterValue) {
        // Borrow the value so we can pattern-match multiple times without moving it.
        let vref = &value;
        prop_if_chains!(id, self, vref,
            (PROP_ENABLED, FilterValue::Bool, PORT_ENABLED, bool, enabled),
            (PROP_MODE_LEFT, FilterValue::Int32, PORT_MODE_LEFT, i32, mode_left),
            (PROP_MODE_RIGHT, FilterValue::Int32, PORT_MODE_RIGHT, i32, mode_right),
            (PROP_TIME_LEFT, FilterValue::Float32, PORT_TIME_LEFT, f32, time_left),
            (PROP_TIME_RIGHT, FilterValue::Float32, PORT_TIME_RIGHT, f32, time_right),
        );

        // If we reach here, no arm matched the id/variant combination.
        warn!("Attempted to set unsupported property id {} value {:?}", id, value);
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, outputs: Vec<&mut [f32]>) {
        self.plugin.process(inputs, outputs);
    }
}

impl DelayFilter {
    pub fn new(rate: u32, block_size: usize, defaults: DelayDefaults) -> Self {
        let mut s = {
            // We should probably do better here and not panic
            let plugin = match LV2PluginBase::new(LSP_COMP_DELAY_URI, rate, block_size) {
                Ok(p) => p,
                Err(e) => {
                    panic!("LV2 Failed '{}': {}", LSP_COMP_DELAY_URI, e);
                }
            };

            Self {
                plugin,
                enabled: defaults.enabled,
                mode_left: defaults.mode_left,
                mode_right: defaults.mode_right,
                time_left: defaults.time_left,
                time_right: defaults.time_right,
            }
        };

        // Apply the defaults to the plugin controls
        let d = defaults;
        s.plugin
            .set_typed_prop(PORT_ENABLED, d.enabled, |v| s.enabled = v);
        s.plugin
            .set_typed_prop(PORT_MODE_LEFT, d.mode_left, |v| s.mode_left = v);
        s.plugin
            .set_typed_prop(PORT_MODE_RIGHT, d.mode_right, |v| s.mode_right = v);
        s.plugin
            .set_typed_prop(PORT_TIME_LEFT, d.time_left, |v| s.time_left = v);
        s.plugin
            .set_typed_prop(PORT_TIME_RIGHT, d.time_right, |v| s.time_right = v);

        s
    }
}

/// Default values for DelayFilter properties, can be constructed using the builder.
#[derive(Debug, Clone)]
pub struct DelayDefaults {
    pub enabled: bool,
    pub mode_left: i32,
    pub mode_right: i32,
    pub time_left: f32,
    pub time_right: f32,
}

impl Default for DelayDefaults {
    fn default() -> Self {
        Self {
            enabled: true,
            mode_left: 2,
            mode_right: 2,
            time_left: 1000.0,
            time_right: 1000.0,
        }
    }
}

/// Builder for `DelayDefaults` so callers can override only the values they care about.
pub struct DelayDefaultsBuilder(DelayDefaults);

impl DelayDefaults {
    pub fn builder() -> DelayDefaultsBuilder {
        DelayDefaultsBuilder(DelayDefaults::default())
    }
}

impl DelayDefaultsBuilder {
    pub fn enabled(mut self, v: bool) -> Self {
        self.0.enabled = v;
        self
    }
    pub fn mode_left(mut self, v: i32) -> Self {
        self.0.mode_left = v;
        self
    }
    pub fn mode_right(mut self, v: i32) -> Self {
        self.0.mode_right = v;
        self
    }
    pub fn time_left(mut self, v: f32) -> Self {
        self.0.time_left = v;
        self
    }
    pub fn time_right(mut self, v: f32) -> Self {
        self.0.time_right = v;
        self
    }
    pub fn build(self) -> DelayDefaults {
        self.0
    }
}

pub fn filter_get_delay_props(name: String, id: Ulid) -> FilterProperties {
    let description = name.to_lowercase().replace(" ", "-");
    let defaults = DelayDefaults::builder().build();

    FilterProperties {
        filter_id: id,
        filter_name: "Delay".to_string(),
        filter_nick: name.to_string(),
        filter_description: format!("{}/{}", APP_NAME_ID, description),

        class: MediaClass::Duplex,
        app_id: APP_ID.to_string(),
        app_name: APP_NAME.to_string(),
        linger: false,

        callback: Box::new(DelayFilter::new(96000, 1024, defaults)),
        ready_sender: None,
    }
}
