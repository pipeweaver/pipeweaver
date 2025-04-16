use pipeweaver_pipewire::{FilterHandler, FilterProperty, FilterValue};

pub struct MeterFilter {}

impl MeterFilter {
    pub(crate) fn new() -> Self {
        Self {}
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
        // Outputs will be empty in this case, but we need to take the input samples from stereo
        // and convert them to a mono input, so we can calculate a percentage. We are going to
        // HARD assume that inputs has 2 entries, left and right (this is how the filter should
        // be created), so we'll generate an average from it. We're also assuming that the number
        // of samples coming on the left and right side is identical

        let samples: Vec<f32> = inputs[0].iter().zip(inputs[1].iter()).map(|(l, r)| (l + r) / 2.0).collect();

        // Use a RMS calc to work out what our 'volume' level is
        let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt(); // RMS calculation

        if rms == 0.0 {
            // Silence Case, we're at 0%, we need to bail here to prevent a divide by zero :D
        }

        let db = 20.0 * rms.log10(); // Convert to dB
        let meter = ((db + 60.0) / 60.0 * 100.0).clamp(0.0, 100.0) as u8; // Normalize to a percentage

        // We can meter as u8 here to get a 'percentage'
    }
}