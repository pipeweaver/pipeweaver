use pipecast_pipewire::{FilterHandler, FilterProperty, FilterValue};

pub struct VolumeFilter {
    volume: u8,
    volume_inner: f32,
}

impl VolumeFilter {
    pub(crate) fn new(volume: u8) -> Self {
        // Grab and clamp the volumes
        let (volume, volume_inner) = if volume >= 100 {
            (100, 1.)
        } else {
            (volume, volume as f32 / 100.)
        };

        Self {
            volume,
            volume_inner,
        }
    }
}

impl FilterHandler for VolumeFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![FilterProperty {
            id: 0,
            name: "Volume".into(),
            value: FilterValue::UInt8(self.volume)
        }]
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        match id {
            0 => FilterProperty {
                id: 0,
                name: "Volume".into(),
                value: FilterValue::UInt8(self.volume),
            },
            _ => panic!("Attempted to lookup non-existent property!")
        }
    }

    fn set_property(&mut self, id: u32, value: FilterValue) {
        match id {
            0 => {
                if let FilterValue::UInt8(value) = value {
                    // Clamp the Max value to 100
                    if value >= 100 {
                        self.volume = 100;
                        self.volume_inner = 1.;
                    } else {
                        self.volume = value;
                        self.volume_inner = value as f32 / 100.;
                    }
                } else {
                    panic!("Attempted to Set Volume as non-percentage");
                }
            }
            _ => panic!("Attempted to set non-existent property!")
        }
    }

    fn process_samples(&self, inputs: Vec<&mut [f32]>, mut outputs: Vec<&mut [f32]>) {
        for (i, input) in inputs.iter().enumerate() {
            if input.is_empty() || outputs[i].is_empty() {
                continue;
            }

            // If we're at max volume, just pass through to the output
            if self.volume_inner == 1. {
                outputs[i].copy_from_slice(input);
                continue;
            }

            // Otherwise, multiply the samples by the volume
            for (index, sample) in input.iter().enumerate() {
                outputs[i][index] = sample * self.volume_inner;
            }
        }
    }
}