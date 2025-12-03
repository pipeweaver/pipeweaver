use pipeweaver_pipewire::{FilterHandler, FilterProperty, FilterValue};

pub struct PassThroughFilter {}

impl PassThroughFilter {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl FilterHandler for PassThroughFilter {
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
        for (i, input) in inputs.iter().enumerate() {
            if input.is_empty() || outputs[i].is_empty() {
                continue;
            }
            outputs[i].copy_from_slice(input);
        }
    }
}
