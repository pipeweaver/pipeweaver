use anyhow::{Result, bail};
use pipeweaver_pipewire::{FilterHandler, FilterProperty, FilterValue};
use tokio::sync::mpsc;
use ulid::Ulid;

// Power Factor is inherited from audio_filters/volume.rs
const POWER_FACTOR: f32 = 3.8;
const INV_POWER_FACTOR: f32 = 1.0 / POWER_FACTOR; // Precompute inverse

// The frequency we should send events upstream
const MILLISECONDS: u32 = 100;

const PROP_ENABLED: u32 = 0;

pub struct MeterFilter {
    enabled: bool,

    chunk_size: usize,

    count: usize,
    peak: f32,

    node_id: Ulid,
    callback: mpsc::Sender<(Ulid, u8)>,
}

impl MeterFilter {
    pub(crate) fn new(
        node_id: Ulid,
        callback: mpsc::Sender<(Ulid, u8)>,
        enabled: bool,
        rate: u32,
    ) -> Self {
        let chunk_size = ((rate / 1000) * MILLISECONDS) as usize;

        Self {
            enabled,
            chunk_size,

            count: 0,
            peak: 0.0,

            node_id,
            callback,
        }
    }
}

impl FilterHandler for MeterFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![self.get_property(0)]
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        match id {
            PROP_ENABLED => FilterProperty {
                id: PROP_ENABLED,
                name: "Enabled".into(),
                symbol: "enabled".into(),
                value: FilterValue::Bool(self.enabled),

                min: 0.0,
                max: 1.0,

                enum_def: None,
            },
            _ => panic!("Attempted to lookup non-existent property!"),
        }
    }

    fn set_property(&mut self, id: u32, value: FilterValue) -> Result<String> {
        match id {
            PROP_ENABLED => {
                if let FilterValue::Bool(value) = value {
                    self.enabled = value;
                    Ok("enabled".into())
                } else {
                    bail!("Attempted to Toggle Meter without Bool type");
                }
            }
            _ => bail!("Attempted to set non-existent property!"),
        }
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, mut _outputs: Vec<&mut [f32]>) {
        if !self.enabled || inputs.is_empty() {
            return;
        }

        // Fast path: update peak with optimized calculation
        let peak = self.peak_amplitude(&inputs);
        self.peak = self.peak.max(peak);
        self.count += inputs[0].len();

        if self.count >= self.chunk_size {
            let meter = self.calculate_meter(self.peak);

            // Always send meter updates every 100ms to maintain UI meter decay
            let _ = self.callback.blocking_send((self.node_id, meter));

            // Reset our values
            self.peak = 0.0;
            self.count -= self.chunk_size;
        }
    }
}

impl MeterFilter {
    fn peak_amplitude(&self, inputs: &[&mut [f32]]) -> f32 {
        let mut global_peak = 0.0_f32;

        for channel in inputs {
            if channel.is_empty() {
                continue;
            }

            let mut channel_peak = 0.0_f32;
            for &sample in channel.iter().step_by(16) {
                channel_peak = channel_peak.max(sample.abs());
            }

            global_peak = global_peak.max(channel_peak);
        }

        global_peak
    }

    #[inline]
    fn calculate_meter(&self, peak: f32) -> u8 {
        if peak <= 1e-9 {
            return 0;
        }

        // Use the precomputed inverse power factor
        let meter = 100.0 * peak.powf(INV_POWER_FACTOR);

        // Clamp is often optimized by the compiler, but we can be explicit
        if meter >= 100.0 {
            100
        } else if meter <= 0.0 {
            0
        } else {
            meter as u8
        }
    }
}
