use anyhow::Result;
use pipeweaver_pipewire::{FilterHandler, FilterProperty, FilterValue};
use rt_ring::{Consumer, Producer};

// Capture Side, should be linked to the physical device
pub struct BridgeCaptureFilter {
    producers: Vec<Producer>, // one per channel
}

impl BridgeCaptureFilter {
    pub fn new(producers: Vec<Producer>) -> Self {
        Self { producers }
    }
}

impl FilterHandler for BridgeCaptureFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![]
    }
    fn get_property(&self, _: u32) -> FilterProperty {
        panic!("Attempted to get non-existent property");
    }
    fn set_property(&mut self, _: u32, _: FilterValue) -> Result<String> {
        anyhow::bail!("Attempted to set non-existent property");
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, _outputs: Vec<&mut [f32]>) {
        for (i, input) in inputs.iter().enumerate() {
            if input.is_empty() {
                continue;
            }
            self.producers[i].push_slice(input);
        }
    }
}

// Playback side, should be linked to the pipeweaver node tree
pub struct BridgePlaybackFilter {
    consumers: Vec<Consumer>, // one per channel
}

impl BridgePlaybackFilter {
    pub fn new(consumers: Vec<Consumer>) -> Self {
        Self { consumers }
    }
}

impl FilterHandler for BridgePlaybackFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![]
    }
    fn get_property(&self, _: u32) -> FilterProperty {
        panic!("Attempted to get non-existent property");
    }
    fn set_property(&mut self, _: u32, _: FilterValue) -> Result<String> {
        anyhow::bail!("Attempted to set non-existent property");
    }

    fn process_samples(&mut self, _inputs: Vec<&mut [f32]>, mut outputs: Vec<&mut [f32]>) {
        for (i, output) in outputs.iter_mut().enumerate() {
            if output.is_empty() {
                continue;
            }
            let _ = self.consumers[i].pop_slice(output);
        }
    }
}
