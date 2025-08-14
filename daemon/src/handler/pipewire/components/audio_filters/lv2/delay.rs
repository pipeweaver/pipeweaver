use crate::handler::pipewire::components::audio_filters::lv2::LV2Wrapper;
use parking_lot::Mutex;
use pipeweaver_pipewire::{FilterHandler, FilterProperty, FilterValue};
use std::sync::Arc;

static PLUGIN_NAME: &str = "DELAY";

pub struct DelayFilter {
    host: Arc<Mutex<LV2Wrapper>>,
}

impl DelayFilter {
    pub(crate) fn new() -> Self {
        let mut host = LV2Wrapper::new(1, 512, 48000);
        host.add_plugin(
            "http://lsp-plug.in/plugins/lv2/comp_delay_x2_stereo",
            PLUGIN_NAME.to_owned(),
        )
            .expect("Failed to load plugin");

        host.set_value(PLUGIN_NAME, "Enabled", 1.0);
        host.set_value(PLUGIN_NAME, "Mode Left", 2.0);
        host.set_value(PLUGIN_NAME, "Mode Right", 2.0);
        host.set_value(PLUGIN_NAME, "Time Left", 1000.);


        Self {
            host: Arc::new(Mutex::new(host)),
        }
    }
}

impl FilterHandler for DelayFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![]
    }

    fn get_property(&self, _: u32) -> FilterProperty {
        panic!("Attempted to get non-existent property");
    }

    fn set_property(&mut self, _: u32, _: FilterValue) {
        panic!("Attempted to set non-existent property");
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, mut outputs: Vec<&mut [f32]>) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        // We need to make sure both channels are the same length
        if (inputs[0].len() != inputs[1].len()) || (outputs[0].len() != outputs[1].len()) {
            return;
        }

        // We also need to be sure the input count matches the output length
        if (inputs[0].len() != outputs[0].len()) || (inputs[1].len() != outputs[1].len()) {
            return;
        }

        // We have to assume a stereo input here
        let l = &inputs[0];
        let r = &inputs[1];

        let mut host = self.host.lock();
        let out = host.apply_multi(0, vec![(0, [0, 0, 0])], [l, r]);

        if let Ok(out) = out {
            outputs[0].copy_from_slice(out[0]);
            outputs[1].copy_from_slice(out[1]);
        }
    }
}
