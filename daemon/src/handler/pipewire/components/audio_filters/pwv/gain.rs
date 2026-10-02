use crate::{APP_ID, APP_NAME, APP_NAME_ID};
use anyhow::bail;
use log::debug;
use pipeweaver_pipewire::{FilterHandler, FilterProperties, MediaClass};
use pipeweaver_shared::{FilterProperty, FilterState, FilterValue};
use std::collections::HashMap;
use std::str::FromStr;
use ulid::Ulid;

pub struct GainFilter {
    enabled: bool,
    amount: f32,
}

impl GainFilter {
    pub(crate) fn new(values: HashMap<String, FilterValue>) -> Self {
        let amount = values.get("amount").unwrap_or(&FilterValue::Float32(0.0));
        let amount = match amount {
            FilterValue::Float32(amount) => *amount,
            _ => 0.0,
        }
        .clamp(-36.0, 36.0);

        let enabled = values.get("enabled").unwrap_or(&FilterValue::Bool(true));
        let enabled = match enabled {
            FilterValue::Bool(value) => *value,
            _ => true,
        };

        Self { enabled, amount }
    }
}

impl FilterHandler for GainFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![
            self.get_property(GainProperties::Enabled as u32),
            self.get_property(GainProperties::Amount as u32),
        ]
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        let prop = GainProperties::try_from(id).expect("Invalid property ID");
        match prop {
            GainProperties::Enabled => FilterProperty {
                id: GainProperties::Enabled as u32,
                name: "Enabled".to_string(),
                symbol: "enabled".to_string(),
                value: FilterValue::Bool(self.enabled),

                min: 0.0,
                max: 1.0,

                is_input: false,
                enum_def: None,
            },

            GainProperties::Amount => FilterProperty {
                id: GainProperties::Amount as u32,
                name: "Amount".to_string(),
                symbol: "amount".to_string(),
                value: FilterValue::Float32(self.amount),

                min: -36.0,
                max: 36.0,

                is_input: false,
                enum_def: None,
            },
        }
    }

    fn set_property(&mut self, id: u32, value: FilterValue) -> anyhow::Result<String> {
        let prop = GainProperties::try_from(id).expect("Invalid property ID");
        match prop {
            GainProperties::Enabled => {
                if let FilterValue::Bool(value) = value {
                    self.enabled = value;
                    Ok("enabled".into())
                } else {
                    bail!("Attempted to Set Enabled as non-boolean");
                }
            }
            GainProperties::Amount => {
                if let FilterValue::Float32(value) = value {
                    self.amount = value.clamp(-36.0, 36.0);
                    Ok("amount".into())
                } else {
                    bail!("Attempted to Set Volume as non-decibel float");
                }
            }
        }
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, mut outputs: Vec<&mut [f32]>) {
        for (i, input) in inputs.iter().enumerate() {
            if input.is_empty() || outputs[i].is_empty() {
                continue;
            }
            if self.enabled {
                let gain = 10.0_f32.powf(self.amount / 20.0);
                for (input_sample, output_sample) in input.iter().zip(outputs[i].iter_mut()) {
                    *output_sample = *input_sample * gain;
                }
            } else {
                outputs[i].copy_from_slice(input);
            }
        }
    }
}

pub fn filter_gain(
    id: Ulid,
    name: String,
    defaults: HashMap<String, FilterValue>,
) -> Result<(String, FilterProperties), FilterState> {
    let filter_name = "Gain".to_string();
    let filter_desc = name.to_lowercase().replace(" ", "-");
    debug!("Filter Name: {}", filter_name);

    // Some plugins may have specific defaults, merge them with the incoming map
    let callback = GainFilter::new(defaults);

    let props = FilterProperties {
        filter_id: id,
        filter_name: filter_name.to_string(),
        filter_nick: filter_name.to_string(),
        filter_description: format!("{}/{}", APP_NAME_ID, filter_desc),

        class: MediaClass::Duplex,
        app_id: APP_ID.to_string(),
        app_name: APP_NAME.to_string(),
        linger: false,

        callback: Box::new(callback),
        ready_sender: None,
    };

    Ok((filter_name, props))
}

#[derive(Debug, PartialEq)]
#[repr(u32)]
enum GainProperties {
    Enabled = 0,
    Amount = 1,
}

impl FromStr for GainProperties {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Enabled" => Ok(GainProperties::Enabled),
            "Amount" => Ok(GainProperties::Amount),
            _ => Err(format!("Unknown variant: {s}")),
        }
    }
}

impl TryFrom<u32> for GainProperties {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(GainProperties::Enabled),
            1 => Ok(GainProperties::Amount),
            _ => Err(format!("Invalid Value: {value}")),
        }
    }
}
