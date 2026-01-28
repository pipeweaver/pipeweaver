use pipeweaver_pipewire::{FilterHandler, FilterProperty, FilterValue};

const POWER_FACTOR: f32 = 3.8;

// This buffer exists to optimise the 0% behaviour, .copy_from_slice is much faster than .fill
const ZERO_BUFFER_SIZE: usize = 4096;
static ZERO_BUFFER: [f32; ZERO_BUFFER_SIZE] = [0.0; ZERO_BUFFER_SIZE];

const PROP_VOLUME: u32 = 0;

pub struct VolumeFilter {
    volume: u8,
    volume_inner: f32,
}

impl VolumeFilter {
    pub(crate) fn new(volume: u8) -> Self {
        let (volume, volume_inner) = Self::calculate_volume(volume);
        Self {
            volume,
            volume_inner,
        }
    }

    #[inline]
    fn calculate_volume(volume: u8) -> (u8, f32) {
        if volume >= 100 {
            (100, 1.0)
        } else if volume == 0 {
            (0, 0.0)
        } else {
            let change = 20.0 * (volume as f32 / 100.0).powf(POWER_FACTOR).log10();
            let scale = 10.0_f32.powf(change / 20.0);
            (volume, scale)
        }
    }

    #[inline]
    fn zero_output(output: &mut [f32]) {
        let len = output.len();

        // Use larger chunks for better performance
        if len <= ZERO_BUFFER.len() {
            output.copy_from_slice(&ZERO_BUFFER[..len]);
        } else {
            // For very large buffers, use chunked copying
            let mut remaining = output;
            while remaining.len() > ZERO_BUFFER.len() {
                let (chunk, rest) = remaining.split_at_mut(ZERO_BUFFER.len());
                chunk.copy_from_slice(&ZERO_BUFFER);
                remaining = rest;
            }
            if !remaining.is_empty() {
                remaining.copy_from_slice(&ZERO_BUFFER[..remaining.len()]);
            }
        }
    }

    // Been doing benchmarking (including SIMD), this seems the most optimal
    #[inline]
    fn apply_volume_scalar(&self, input: &[f32], output: &mut [f32]) {
        let volume = self.volume_inner;

        for (out, &inp) in output.iter_mut().zip(input.iter()) {
            *out = inp * volume;
        }
    }
}

impl FilterHandler for VolumeFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![self.get_property(PROP_VOLUME)]
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        match id {
            PROP_VOLUME => FilterProperty {
                id: PROP_VOLUME,
                name: "Volume".into(),
                value: FilterValue::UInt8(self.volume),

                min: 0.0,
                max: 100.0,

                enum_def: None,
            },
            _ => panic!("Attempted to lookup non-existent property!"),
        }
    }

    fn set_property(&mut self, id: u32, value: FilterValue) {
        match id {
            0 => {
                if let FilterValue::UInt8(value) = value {
                    // Clamp the Max value to 100
                    let (volume, volume_inner) = Self::calculate_volume(value);
                    self.volume = volume;
                    self.volume_inner = volume_inner;
                } else {
                    panic!("Attempted to Set Volume as non-percentage");
                }
            }
            _ => panic!("Attempted to set non-existent property!"),
        }
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, mut outputs: Vec<&mut [f32]>) {
        match self.volume_inner {
            1.0 => {
                for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
                    if input.len() == output.len() && !input.is_empty() {
                        output.copy_from_slice(input);
                    }
                }
            }
            0.0 => {
                for output in outputs.iter_mut() {
                    Self::zero_output(output);
                }
            }
            _ => {
                // Apply volume scaling
                for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
                    if input.len() == output.len() && !input.is_empty() {
                        self.apply_volume_scalar(input, output);
                    }
                }
            }
        }
    }
}
