use pipeweaver_pipewire::{FilterHandler, FilterProperty, FilterValue};
use tokio::sync::mpsc;
use ulid::Ulid;

// Power Factor is inherited from audio_filters/volume.rs
const POWER_FACTOR: f32 = 3.8;

// This is how we should be setup
const SAMPLE_RATE: u32 = 48000;

// The frequency we should send events upstream
const MILLISECONDS: u32 = 100;

// The number of samples which should represent a MILLISECONDS time period
const CHUNK_SIZE: usize = ((SAMPLE_RATE / 1000) * MILLISECONDS) as usize;

pub struct MeterFilter {
    count: usize,
    peak: f32,

    node_id: Ulid,
    callback: mpsc::Sender<(Ulid, u8)>,
}

impl MeterFilter {
    pub(crate) fn new(node_id: Ulid, callback: mpsc::Sender<(Ulid, u8)>) -> Self {
        Self {
            count: 0,
            peak: 0.0,

            node_id,
            callback,
        }
    }
}

impl FilterHandler for MeterFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![]
    }

    fn get_property(&self, _: u32) -> FilterProperty {
        panic!("Attempted to get non-existent property");
    }

    fn set_property(&mut self, _: u32, _: FilterValue) {
        panic!("Attempted to set non-existent property");
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, mut _outputs: Vec<&mut [f32]>) {
        // While we're expecting stereo here, we'll support arbitrary channel numbers
        let peak = self.peak_amplitude(&inputs);

        self.peak = self.peak.max(peak);
        self.count += inputs[0].len();

        if self.count >= CHUNK_SIZE {
            let peak = self.peak;

            // We calculate the gain percent by doing the inverse of what the volume filter does,
            // meaning that if you reduce a source channels volume by 50%, it's correctly 
            // represented on the output channels
            let gain = peak.max(1e-9);
            let meter = (100.0 * gain.powf(1.0 / POWER_FACTOR)).clamp(0.0, 100.0) as u8;

            let _ = self.callback.blocking_send((self.node_id, meter));

            // Reset our values
            self.peak = 0.0;
            self.count -= CHUNK_SIZE
        }
    }
}

impl MeterFilter {
    fn peak_amplitude(&self, inputs: &[&mut [f32]]) -> f32 {
        if inputs.is_empty() || inputs[0].is_empty() {
            return 0.0;
        }

        let mut peak = 0.0_f32;
        for i in 0..inputs[0].len() {
            let mut frame_peak = 0.0_f32;
            for channel in inputs {
                frame_peak = frame_peak.max(channel[i].abs());
            }
            peak = peak.max(frame_peak);
        }

        peak
    }
}
