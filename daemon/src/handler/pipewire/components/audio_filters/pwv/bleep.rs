use crate::{APP_ID, APP_NAME, APP_NAME_ID};
use anyhow::bail;
use log::debug;
use pipeweaver_pipewire::{FilterHandler, FilterProperties, MediaClass};
use pipeweaver_shared::{FilterProperty, FilterState, FilterValue};
use std::collections::HashMap;
use std::str::FromStr;
use ulid::Ulid;

const DEFAULT_FREQUENCY: f32 = 1000.0;
const DEFAULT_AMPLITUDE: f32 = 0.5;

// Crossfade duration in milliseconds.
const FADE_MS: f32 = 5.0;

pub struct BleepFilter {
    enabled: bool,
    frequency: f32,
    amplitude: f32,
    sample_rate: u32,

    mix: f32,
    phase: f32,
}

impl BleepFilter {
    pub(crate) fn new(values: HashMap<String, FilterValue>, sample_rate: u32) -> Self {
        let sample_rate = if sample_rate > 0 { sample_rate } else { 48_000 };

        let frequency = values
            .get("frequency")
            .unwrap_or(&FilterValue::Float32(DEFAULT_FREQUENCY));
        let frequency = match frequency {
            FilterValue::Float32(value) if value.is_finite() => *value,
            _ => DEFAULT_FREQUENCY,
        }
        .clamp(20.0, Self::max_frequency(sample_rate));

        let amplitude = values
            .get("amplitude")
            .unwrap_or(&FilterValue::Float32(DEFAULT_AMPLITUDE));
        let amplitude = match amplitude {
            FilterValue::Float32(value) if value.is_finite() => *value,
            _ => DEFAULT_AMPLITUDE,
        }
        .clamp(0.0, 1.0);

        let enabled = values.get("enabled").unwrap_or(&FilterValue::Bool(false));
        let enabled = match enabled {
            FilterValue::Bool(value) => *value,
            _ => false,
        };

        Self {
            enabled,
            frequency,
            amplitude,
            sample_rate,

            // Start fully on the bleep if enabled.
            mix: if enabled { 1.0 } else { 0.0 },

            phase: 0.0,
        }
    }

    fn max_frequency(sample_rate: u32) -> f32 {
        // Keep the frequency below Nyquist.
        (sample_rate as f32 * 0.5 - 1.0).max(20.0)
    }

    fn fade_samples(&self) -> f32 {
        (self.sample_rate as f32 * (FADE_MS / 1000.0)).max(1.0)
    }
}

impl FilterHandler for BleepFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![
            self.get_property(BleepProperties::Enabled as u32),
            self.get_property(BleepProperties::Frequency as u32),
            self.get_property(BleepProperties::Amplitude as u32),
        ]
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        let prop = BleepProperties::try_from(id).expect("Invalid property ID");

        match prop {
            BleepProperties::Enabled => FilterProperty {
                id: BleepProperties::Enabled as u32,
                name: "Enabled".to_string(),
                symbol: "enabled".to_string(),
                value: FilterValue::Bool(self.enabled),

                min: 0.0,
                max: 1.0,

                is_input: false,
                enum_def: None,
            },

            BleepProperties::Frequency => FilterProperty {
                id: BleepProperties::Frequency as u32,
                name: "Frequency".to_string(),
                symbol: "frequency".to_string(),
                value: FilterValue::Float32(self.frequency),

                min: 20.0,
                max: Self::max_frequency(self.sample_rate),

                is_input: false,
                enum_def: None,
            },

            BleepProperties::Amplitude => FilterProperty {
                id: BleepProperties::Amplitude as u32,
                name: "Amplitude".to_string(),
                symbol: "amplitude".to_string(),
                value: FilterValue::Float32(self.amplitude),

                min: 0.0,
                max: 1.0,

                is_input: false,
                enum_def: None,
            },
        }
    }

    fn set_property(&mut self, id: u32, value: FilterValue) -> anyhow::Result<String> {
        let prop = BleepProperties::try_from(id).expect("Invalid property ID");

        match prop {
            BleepProperties::Enabled => {
                if let FilterValue::Bool(value) = value {
                    self.enabled = value;
                    Ok("enabled".into())
                } else {
                    bail!("Attempted to Set Enabled as non-boolean");
                }
            }

            BleepProperties::Frequency => {
                if let FilterValue::Float32(value) = value {
                    if !value.is_finite() {
                        bail!("Attempted to Set Frequency as non-finite float");
                    }

                    self.frequency = value.clamp(20.0, Self::max_frequency(self.sample_rate));

                    Ok("frequency".into())
                } else {
                    bail!("Attempted to Set Frequency as non-float");
                }
            }

            BleepProperties::Amplitude => {
                if let FilterValue::Float32(value) = value {
                    if !value.is_finite() {
                        bail!("Attempted to Set Amplitude as non-finite float");
                    }

                    self.amplitude = value.clamp(0.0, 1.0);

                    Ok("amplitude".into())
                } else {
                    bail!("Attempted to Set Amplitude as non-float");
                }
            }
        }
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, mut outputs: Vec<&mut [f32]>) {
        if outputs.is_empty() || inputs.is_empty() {
            return;
        }

        if !self.enabled && self.mix <= 0.0 {
            for (i, input) in inputs.iter().enumerate() {
                if input.is_empty() || outputs[i].is_empty() {
                    continue;
                }
                outputs[i].copy_from_slice(input);
            }
            return;
        }

        let two_pi = 2.0 * std::f32::consts::PI;
        let phase_increment = two_pi * self.frequency / self.sample_rate as f32;

        let fade_samples = self.fade_samples();
        let mix_step = 1.0 / fade_samples;

        let target_mix = if self.enabled { 1.0 } else { 0.0 };
        let frame_count = outputs.iter().map(|output| output.len()).max().unwrap_or(0);

        for frame in 0..frame_count {
            // Advance the Crossfade
            if self.mix < target_mix {
                self.mix = (self.mix + mix_step).min(target_mix);
            } else if self.mix > target_mix {
                self.mix = (self.mix - mix_step).max(target_mix);
            }

            // Generate a sample for this frame
            let bleep = self.phase.sin() * self.amplitude;
            self.phase += phase_increment;

            if self.phase >= two_pi {
                self.phase -= two_pi;
            }

            // Apply the bleep to the input
            for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
                output[frame] = input[frame] * (1.0 - self.mix) + bleep * self.mix;
            }
        }

        // Reset the oscillator once the filter has completely faded out
        if !self.enabled && self.mix <= 0.0 {
            self.mix = 0.0;
            self.phase = 0.0;
        }
    }
}

pub fn filter_bleep(
    id: Ulid,
    name: String,
    defaults: HashMap<String, FilterValue>,
    sample_rate: u32,
) -> Result<(String, FilterProperties), FilterState> {
    let filter_name = "Bleep".to_string();
    let filter_desc = name.to_lowercase().replace(" ", "-");

    debug!("Filter Name: {}", filter_name);

    let callback = BleepFilter::new(defaults, sample_rate);

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
enum BleepProperties {
    Enabled = 0,
    Frequency = 1,
    Amplitude = 2,
}

impl FromStr for BleepProperties {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Enabled" => Ok(BleepProperties::Enabled),
            "Frequency" => Ok(BleepProperties::Frequency),
            "Amplitude" => Ok(BleepProperties::Amplitude),
            _ => Err(format!("Unknown variant: {s}")),
        }
    }
}

impl TryFrom<u32> for BleepProperties {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BleepProperties::Enabled),
            1 => Ok(BleepProperties::Frequency),
            2 => Ok(BleepProperties::Amplitude),
            _ => Err(format!("Invalid Value: {value}")),
        }
    }
}
