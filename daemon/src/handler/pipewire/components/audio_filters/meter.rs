use pipeweaver_pipewire::{FilterHandler, FilterProperty, FilterValue};
use tokio::sync::mpsc;
use ulid::Ulid;

// Power Factor is inherited from audio_filters/volume.rs
const POWER_FACTOR: f32 = 3.8;
const INV_POWER_FACTOR: f32 = 1.0 / POWER_FACTOR; // Precompute inverse

// This is how we should be setup
const SAMPLE_RATE: u32 = 48000;

// The frequency we should send events upstream
const MILLISECONDS: u32 = 100;

// The number of samples which should represent a MILLISECONDS time period
const CHUNK_SIZE: usize = ((SAMPLE_RATE / 1000) * MILLISECONDS) as usize;

pub struct MeterFilter {
    enabled: bool,

    count: usize,
    peak: f32,

    node_id: Ulid,
    callback: mpsc::Sender<(Ulid, u8)>,
}

impl MeterFilter {
    pub(crate) fn new(node_id: Ulid, callback: mpsc::Sender<(Ulid, u8)>) -> Self {
        Self {
            enabled: true,

            count: 0,
            peak: 0.0,

            node_id,
            callback,
        }
    }
}

impl FilterHandler for MeterFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![FilterProperty {
            id: 0,
            name: "Enabled".into(),
            value: FilterValue::Bool(self.enabled)
        }]
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        match id {
            0 => FilterProperty {
                id: 0,
                name: "Volume".into(),
                value: FilterValue::Bool(self.enabled),
            },
            _ => panic!("Attempted to lookup non-existent property!")
        }
    }

    fn set_property(&mut self, id: u32, value: FilterValue) {
        match id {
            0 => {
                if let FilterValue::Bool(value) = value {
                    self.enabled = value;
                } else {
                    panic!("Attempted to Toggle Meter without Bool type");
                }
            }
            _ => panic!("Attempted to set non-existent property!")
        }
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, mut _outputs: Vec<&mut [f32]>) {
        if inputs.is_empty() {
            return;
        }

        // Fast path: update peak with optimized calculation
        let peak = self.peak_amplitude(&inputs);
        self.peak = self.peak.max(peak);
        self.count += inputs[0].len();

        if self.count >= CHUNK_SIZE {
            let meter = self.calculate_meter(self.peak);

            // Always send meter updates every 100ms to maintain UI meter decay
            let _ = self.callback.blocking_send((self.node_id, meter));

            // Reset our values
            self.peak = 0.0;
            self.count -= CHUNK_SIZE;
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
            for &sample in channel.iter() {
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