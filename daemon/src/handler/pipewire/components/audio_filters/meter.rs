use pipeweaver_pipewire::{FilterHandler, FilterProperty, FilterValue};
use tokio::sync::mpsc;
use ulid::Ulid;

// This is how we should be setup
const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u32 = 2;

// The frequency we should send events upstream
const MILLISECONDS: u32 = 100;

// The number of samples which should represent a MILLISECONDS time period 
const CHUNK_SIZE: usize = ((SAMPLE_RATE * CHANNELS * MILLISECONDS) / 1000) as usize;

// The 0% floor for audio in decibels
const DB_FLOOR: f32 = -60.0;

pub struct MeterFilter {
    node_id: Ulid,
    callback: mpsc::Sender<(Ulid, u8)>,
    buffer: ChunkedBuffer,
}

impl MeterFilter {
    pub(crate) fn new(node_id: Ulid, callback: mpsc::Sender<(Ulid, u8)>) -> Self {
        Self {
            node_id,
            callback,
            buffer: ChunkedBuffer::new(CHUNK_SIZE),
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
        // Outputs will be empty in this case, but we need to take the input samples from stereo.
        // Once we have the samples, we'll determine whether the left or right is louder and use
        // that as our meter sample.
        let samples: Vec<f32> = inputs[0]
            .iter()
            .zip(inputs[1].iter())
            .map(|(l, r)| if l.abs() > r.abs() { *l } else { *r })
            .collect();

        if let Some(values) = self.buffer.push(&samples) {
            // Find the peak sample
            let peak = values.iter().copied().map(f32::abs).fold(0.0, f32::max);

            // Use 1e-9 as a minimum to prevent a 0.log10()
            let db = 20.0 * peak.max(1e-9).log10();
            let meter = (((db - DB_FLOOR) / -DB_FLOOR) * 100.0).clamp(0.0, 100.0) as u8;

            let _ = self.callback.blocking_send((self.node_id, meter));
        }
    }
}

struct ChunkedBuffer {
    buffer: Vec<f32>,
    len: usize,
    chunk_size: usize,
}

impl ChunkedBuffer {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            buffer: vec![0.0; chunk_size],
            len: 0,
            chunk_size,
        }
    }

    pub fn push(&mut self, samples: &[f32]) -> Option<&[f32]> {
        let total_len = self.len + samples.len();

        if total_len >= self.chunk_size {
            let needed = self.chunk_size - self.len;
            self.buffer[self.len..].copy_from_slice(&samples[..needed]);

            // Handle remaining right-hand samples (but discard anything beyond one chunk)
            let remaining = &samples[needed..];
            if !remaining.is_empty() {
                let right_len = remaining.len().min(self.chunk_size);
                let start = remaining.len() - right_len;
                self.buffer[..right_len].copy_from_slice(&remaining[start..]);
                self.len = right_len;
            } else {
                self.len = 0;
            }
            Some(&self.buffer[..self.chunk_size])
        } else {
            self.buffer[self.len..self.len + samples.len()].copy_from_slice(samples);
            self.len += samples.len();
            None
        }
    }
}
