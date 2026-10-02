// RNNoise via nnnoise

use crate::{APP_ID, APP_NAME, APP_NAME_ID};
use anyhow::{anyhow, bail};
use log::debug;
use nnnoiseless::DenoiseState;
use pipeweaver_pipewire::{FilterHandler, FilterProperties, MediaClass};
use pipeweaver_shared::{FilterProperty, FilterState, FilterValue};
use rubato::{Fft, FixedSync, Resampler, audioadapter_buffers::direct::SequentialSliceOfVecs};
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use ulid::Ulid;

const RNNOISE_SAMPLE_RATE: usize = 48_000;
const FRAME: usize = DenoiseState::FRAME_SIZE;

/// RNNoise expects f32 samples scaled to the i16 range.
const PCM_SCALE: f32 = 32768.0;

/// Largest host quantum the FIFOs are pre-sized for (larger still works, but
/// may allocate once).
const MAX_QUANTUM: usize = 8192;
const MAX_ATTENUATION_DB: f32 = 60.0;

// Defaults used when the caller doesn't supply a value.
const DEFAULT_MAX_ATTENUATION_DB: f32 = 30.0;
const DEFAULT_VAD_THRESHOLD: f32 = 0.7;
const DEFAULT_VAD_GRACE_MS: f32 = 200.0;
const DEFAULT_RETRO_VAD_GRACE_MS: f32 = 0.0;
const DEFAULT_MIX: f32 = 1.0;

const DEFAULT_GATE_RANGE_DB: f32 = 60.0;
const DEFAULT_GATE_ATTACK_MS: f32 = 5.0;
const DEFAULT_GATE_RELEASE_MS: f32 = 60.0;
const DEFAULT_SIDE_REDUCTION: f32 = 1.0;

const MAX_VAD_PERIOD_MS: f32 = 1000.0;
const MAX_GATE_RANGE_DB: f32 = 80.0;
const MAX_GATE_TIME_MS: f32 = 500.0;
const MAX_RETRO_FRAMES: usize = 100;

const DELAY_SLOTS: usize = MAX_RETRO_FRAMES + 1;

pub struct NoiseSuppressionFilter {
    // User parameters
    enabled: bool,
    max_attenuation: f32,
    vad_threshold: f32,
    vad_grace_period: f32,
    retroactive_vad_grace_period: f32,
    mix: f32,
    gate_range: f32,
    gate_attack: f32,
    gate_release: f32,
    side_reduction: f32,

    // Values derived from the parameters
    wet_target: f32,
    gate_floor: f32,
    attack_coef: f32,
    release_coef: f32,
    grace_frames: i64,
    retro_frames: usize,

    channels: usize,

    input_resampler: Option<Fft<f32>>,
    output_resampler: Option<Fft<f32>>,

    /// Host-rate accumulation buffer (one chunk for the input resampler).
    input_buffer: Vec<Vec<f32>>,
    input_buffered: usize,

    /// Current 48k input frame.
    frame_in: Vec<Vec<f32>>,
    frame_dry: Vec<Vec<f32>>,
    frame_out: Vec<Vec<f32>>,
    resampled_output: Vec<Vec<f32>>,

    /// RNNoise is a mono model: one denoiser runs on the channel average.
    denoiser: Box<DenoiseState<'static>>,
    mid_wet: Vec<f32>,
    link_weights: Vec<f32>,
    side_gain: f32,
    first_frame: bool,
    wet_applied: f32,

    /// Ring of the last `DELAY_SLOTS` processed frames, laid out as
    /// `[slot][channel][sample]`.
    delay_ring: Vec<f32>,
    delay_head: usize,
    last_speech: Option<i64>,
    frame_index: i64,
    gate_gain: f32,

    output_fifo: Vec<VecDeque<f32>>,
}

impl NoiseSuppressionFilter {
    pub(crate) fn new(
        values: HashMap<String, FilterValue>,
        sample_rate: u32,
        channels: usize,
    ) -> Self {
        let float = |key: &str, default: f32| match values.get(key) {
            Some(FilterValue::Float32(value)) => *value,
            _ => default,
        };

        let enabled = match values.get("enabled") {
            Some(FilterValue::Bool(value)) => *value,
            _ => true,
        };
        let max_attenuation = clamp_param(
            float("max_attenuation", DEFAULT_MAX_ATTENUATION_DB),
            0.0,
            MAX_ATTENUATION_DB,
        );
        let vad_threshold = clamp_param(float("vad_threshold", DEFAULT_VAD_THRESHOLD), 0.0, 1.0);
        let vad_grace_period = clamp_param(
            float("vad_grace_period", DEFAULT_VAD_GRACE_MS),
            0.0,
            MAX_VAD_PERIOD_MS,
        );
        let retroactive_vad_grace_period = clamp_param(
            float("retroactive_vad_grace_period", DEFAULT_RETRO_VAD_GRACE_MS),
            0.0,
            MAX_VAD_PERIOD_MS,
        );
        let mix = clamp_param(float("mix", DEFAULT_MIX), 0.0, 1.0);
        let gate_range = clamp_param(
            float("gate_range", DEFAULT_GATE_RANGE_DB),
            0.0,
            MAX_GATE_RANGE_DB,
        );
        let gate_attack = clamp_param(
            float("gate_attack", DEFAULT_GATE_ATTACK_MS),
            0.0,
            MAX_GATE_TIME_MS,
        );
        let gate_release = clamp_param(
            float("gate_release", DEFAULT_GATE_RELEASE_MS),
            0.0,
            MAX_GATE_TIME_MS,
        );
        let side_reduction = clamp_param(float("side_reduction", DEFAULT_SIDE_REDUCTION), 0.0, 1.0);

        // Build the resamplers if they're needed, rnnoise only works on 48khz streams, so if we're
        // not that already, we need to become that for processing, then jump back to the original.
        let (input_resampler, output_resampler) = if sample_rate as usize == RNNOISE_SAMPLE_RATE {
            (None, None)
        } else {
            let input = Fft::<f32>::new(
                sample_rate as usize,
                RNNOISE_SAMPLE_RATE,
                FRAME,
                channels,
                FixedSync::Output,
            )
            .expect("failed to create input resampler");

            let output = Fft::<f32>::new(
                RNNOISE_SAMPLE_RATE,
                sample_rate as usize,
                FRAME,
                channels,
                FixedSync::Input,
            )
            .expect("failed to create output resampler");

            (Some(input), Some(output))
        };

        let input_max = input_resampler
            .as_ref()
            .map_or(FRAME, |r| r.input_frames_max());
        let output_max = output_resampler
            .as_ref()
            .map_or(FRAME, |r| r.output_frames_max());
        let output_next = output_resampler
            .as_ref()
            .map_or(FRAME, |r| r.output_frames_next());

        // Silence pre-rolled into the output FIFO. Input is consumed in chunks
        // of at most `input_max` host frames, so the FIFO can lag the input by
        // up to that much; the extra term covers a varying output chunk size.
        let latency_pad = input_max + output_max.saturating_sub(output_next);

        let frames =
            |size: usize| -> Vec<Vec<f32>> { (0..channels).map(|_| vec![0.0; size]).collect() };

        let fifo_capacity = latency_pad + MAX_QUANTUM * 2 + output_max;
        let output_fifo = (0..channels)
            .map(|_| {
                let mut fifo = VecDeque::with_capacity(fifo_capacity);
                fifo.extend(std::iter::repeat_n(0.0_f32, latency_pad));
                fifo
            })
            .collect();

        Self {
            enabled,
            max_attenuation,
            vad_threshold,
            vad_grace_period,
            retroactive_vad_grace_period,
            mix,
            gate_range,
            gate_attack,
            gate_release,
            side_reduction,

            wet_target: wet_target_for(max_attenuation, mix),
            gate_floor: gate_floor_for(gate_range),
            attack_coef: smoothing_coef(gate_attack),
            release_coef: smoothing_coef(gate_release),
            grace_frames: ms_to_frames(vad_grace_period) as i64,
            retro_frames: ms_to_frames(retroactive_vad_grace_period).min(MAX_RETRO_FRAMES),

            channels,

            input_resampler,
            output_resampler,

            input_buffer: frames(input_max),
            input_buffered: 0,

            frame_in: frames(FRAME),
            frame_dry: frames(FRAME),
            frame_out: frames(FRAME),
            resampled_output: frames(output_max),

            denoiser: DenoiseState::new(),
            mid_wet: vec![0.0; FRAME],
            link_weights: vec![1.0; channels],
            side_gain: 1.0,
            first_frame: true,
            wet_applied: 0.0,

            delay_ring: vec![0.0; DELAY_SLOTS * channels * FRAME],
            delay_head: 0,
            last_speech: None,
            frame_index: 0,
            gate_gain: 1.0,

            output_fifo,
        }
    }

    /// Accumulate samples, and run full chunks through
    fn process(&mut self, inputs: &[&mut [f32]]) {
        let total = inputs[0].len();
        let mut offset = 0;

        while offset < total {
            let required = match self.input_resampler.as_ref() {
                Some(resampler) => resampler.input_frames_next(),
                None => FRAME,
            };

            let copy = (required - self.input_buffered).min(total - offset);

            for (dst, src) in self.input_buffer.iter_mut().zip(inputs) {
                if src.is_empty() || dst.is_empty() {
                    // This shouldn't happen unless the node is entirely unlinked.
                    return;
                }
                let start = self.input_buffered;
                let end = start + copy;
                // debug!(
                //     "Start: {start} - End {end} - Copy {copy} - Total {total} - Offset {offset} - Dst: {} - Src: {}",
                //     dst.len(),
                //     src.len()
                // );
                dst[start..end].copy_from_slice(&src[offset..offset + copy]);
            }

            self.input_buffered += copy;
            offset += copy;

            if self.input_buffered < required {
                break;
            }
            self.input_buffered = 0;

            self.load_frame(required);
            self.process_frame();
        }
    }

    /// Fills `frame_in` with exactly one 48k frame from `input_buffer`.
    fn load_frame(&mut self, required: usize) {
        match self.input_resampler.as_mut() {
            None => {
                for (dst, src) in self.frame_in.iter_mut().zip(&self.input_buffer) {
                    dst.copy_from_slice(&src[..FRAME]);
                }
            }
            Some(resampler) => {
                let input = SequentialSliceOfVecs::new(&self.input_buffer, self.channels, required)
                    .expect("input buffer is sized at construction");
                let mut output =
                    SequentialSliceOfVecs::new_mut(&mut self.frame_in, self.channels, FRAME)
                        .expect("frame buffer is sized at construction");

                let produced = resampler
                    .process_into_buffer(&input, &mut output, None)
                    .map(|(_, produced)| produced)
                    .unwrap_or(0);

                // Never expected, but don't feed stale data to the denoiser.
                if produced < FRAME {
                    for channel in self.frame_in.iter_mut() {
                        channel[produced..].fill(0.0);
                    }
                }
            }
        }
    }

    fn process_frame(&mut self) {
        // While disabled (and fully faded out) RNNoise isn't run at all.
        let run_inference = self.enabled || self.wet_applied > 0.0;

        // Its first output after (re)start contains fade-in artifacts.
        let discard_wet = !run_inference || self.first_frame;
        let mut vad = 0.0_f32;

        if run_inference {
            // Downmix to mono and denoise once, so all channels share one
            // set of gain decisions.
            let mut scaled = [0.0_f32; FRAME];
            for channel in &self.frame_in {
                for (dst, src) in scaled.iter_mut().zip(channel) {
                    *dst += *src;
                }
            }
            let scale = PCM_SCALE / self.channels as f32;
            for sample in scaled.iter_mut() {
                *sample *= scale;
            }

            vad = self.denoiser.process_frame(&mut self.mid_wet, &scaled);
            for sample in self.mid_wet.iter_mut() {
                *sample /= PCM_SCALE;
            }
        }

        self.first_frame = !run_inference;

        if discard_wet {
            vad = 0.0;
        }

        self.blend(discard_wet);

        for (dry, input) in self.frame_dry.iter_mut().zip(&self.frame_in) {
            dry.copy_from_slice(input);
        }

        self.delay_and_gate(vad);
        self.push_output();
    }

    /// Applies the denoiser's correction to every channel.
    fn blend(&mut self, discard_wet: bool) {
        let start = self.wet_applied;
        let end = if self.enabled { self.wet_target } else { 0.0 };

        if discard_wet || (start == 0.0 && end == 0.0) {
            for (out, dry) in self.frame_out.iter_mut().zip(&self.frame_dry) {
                out.copy_from_slice(dry);
            }
            self.wet_applied = 0.0;
            self.link_weights.fill(1.0);
            self.side_gain = 1.0;
            return;
        }

        let channels = self.channels;
        let rms = |x: &[f32]| (x.iter().map(|s| s * s).sum::<f32>() / FRAME as f32).sqrt();

        // Channel average of the (delayed) dry signal.
        let mut mid_dry = [0.0_f32; FRAME];
        for dry in &self.frame_dry {
            for (m, s) in mid_dry.iter_mut().zip(dry) {
                *m += *s / channels as f32;
            }
        }
        let mid_rms = rms(&mid_dry);
        let k_step = (end - start) / FRAME as f32;

        // The correction and the energy of the corrected mid.
        let mut delta = [0.0_f32; FRAME];
        let mut out_energy = 0.0_f32;
        let mut k = start;
        for ((d, &wet), &mid) in delta.iter_mut().zip(&self.mid_wet).zip(&mid_dry) {
            k += k_step;
            *d = wet - mid;
            let mid_out = mid + *d * k;
            out_energy += mid_out * mid_out;
        }

        // Broadband gain the mid received this frame (<= 1).
        let mid_gain = if mid_rms > 1e-9 {
            ((out_energy / FRAME as f32).sqrt() / mid_rms).min(1.0)
        } else {
            1.0
        };
        let side_start = self.side_gain;
        let side_step = (mid_gain - side_start) / FRAME as f32;
        let side_reduction = self.side_reduction;

        for ch in 0..channels {
            // A channel that is louder than the mid (e.g. mic on one side
            // only) needs a proportionally larger correction.
            let target = if mid_rms > 1e-9 {
                (rms(&self.frame_dry[ch]) / mid_rms).min(channels as f32)
            } else {
                1.0
            };
            let w_step = (target - self.link_weights[ch]) / FRAME as f32;

            let mut k = start;
            let mut w = self.link_weights[ch];
            let mut g = side_start;
            for (((out, &dry), &d), &mid) in self.frame_out[ch]
                .iter_mut()
                .zip(&self.frame_dry[ch])
                .zip(&delta)
                .zip(&mid_dry)
            {
                k += k_step;
                w += w_step;
                g += side_step;

                let residual = dry - w * mid;
                *out = dry + d * w * k - residual * side_reduction * (1.0 - g);
            }
            self.link_weights[ch] = target;
        }

        self.side_gain = mid_gain;
        self.wet_applied = end;
    }

    /// Pushes the frame through the retroactive delay line, then applies the VAD gate
    fn delay_and_gate(&mut self, vad: f32) {
        let gating = self.vad_threshold > 0.0;
        let delay = if gating { self.retro_frames } else { 0 };
        let stride = self.channels * FRAME;

        // Write the new frame, read the one delay frames behind it.
        let write = self.delay_head;
        let read = (write + DELAY_SLOTS - delay) % DELAY_SLOTS;

        for (ch, frame) in self.frame_out.iter().enumerate() {
            let at = write * stride + ch * FRAME;
            self.delay_ring[at..at + FRAME].copy_from_slice(frame);
        }
        for (ch, frame) in self.frame_out.iter_mut().enumerate() {
            let at = read * stride + ch * FRAME;
            frame.copy_from_slice(&self.delay_ring[at..at + FRAME]);
        }
        self.delay_head = (write + 1) % DELAY_SLOTS;

        let open = if gating && self.enabled {
            if vad >= self.vad_threshold {
                self.last_speech = Some(self.frame_index);
            }
            let emitted = self.frame_index - delay as i64;
            matches!(self.last_speech, Some(last) if last + self.grace_frames >= emitted)
        } else {
            self.last_speech = None;
            true
        };
        self.frame_index += 1;

        let target = if open { 1.0 } else { self.gate_floor };

        if self.gate_gain == 1.0 && target == 1.0 {
            // Fully open, nothing to do.
            return;
        }

        // Per-sample smoothing: attack when opening, release when closing.
        let mut gains = [0.0_f32; FRAME];
        let mut gain = self.gate_gain;
        for g in gains.iter_mut() {
            let coef = if target > gain {
                self.attack_coef
            } else {
                self.release_coef
            };
            gain += (target - gain) * coef;
            if (target - gain).abs() < 1e-5 {
                gain = target;
            }
            *g = gain;
        }
        self.gate_gain = gain;

        for frame in self.frame_out.iter_mut() {
            for (sample, &g) in frame.iter_mut().zip(&gains) {
                *sample *= g;
            }
        }
    }

    /// Resamples `frame_out` back to the host rate and queues it.
    fn push_output(&mut self) {
        match self.output_resampler.as_mut() {
            None => {
                for (fifo, frame) in self.output_fifo.iter_mut().zip(&self.frame_out) {
                    fifo.extend(frame.iter());
                }
            }
            Some(resampler) => {
                let input = SequentialSliceOfVecs::new(&self.frame_out, self.channels, FRAME)
                    .expect("frame buffer is sized at construction");

                let output_frames = resampler.output_frames_next();
                let mut output = SequentialSliceOfVecs::new_mut(
                    &mut self.resampled_output,
                    self.channels,
                    output_frames,
                )
                .expect("resample buffer is sized at construction");

                let produced = resampler
                    .process_into_buffer(&input, &mut output, None)
                    .map(|(_, produced)| produced)
                    .unwrap_or(0);

                for (fifo, scratch) in self.output_fifo.iter_mut().zip(&self.resampled_output) {
                    fifo.extend(scratch[..produced].iter());
                }
            }
        }
    }

    fn drain_output(&mut self, outputs: &mut [&mut [f32]]) {
        let frames = outputs[0].len();

        // Shouldn't happen thanks to the pre-roll; keep the queued audio and
        // output silence so we recover on the next call.
        if self.output_fifo[0].len() < frames {
            for output in outputs.iter_mut() {
                output.fill(0.0);
            }
            return;
        }

        for (output, fifo) in outputs.iter_mut().zip(self.output_fifo.iter_mut()) {
            for (dst, src) in output.iter_mut().zip(fifo.drain(..frames)) {
                *dst = src;
            }
        }
    }
}

fn clamp_param(value: f32, min: f32, max: f32) -> f32 {
    if value.is_nan() {
        min
    } else {
        value.clamp(min, max)
    }
}

fn ms_to_frames(milliseconds: f32) -> usize {
    (milliseconds * RNNOISE_SAMPLE_RATE as f32 / (FRAME as f32 * 1000.0)).round() as usize
}

fn wet_target_for(max_attenuation: f32, mix: f32) -> f32 {
    let floor_gain = 10.0_f32.powf(-max_attenuation / 20.0);
    (1.0 - floor_gain) * mix
}

fn gate_floor_for(range_db: f32) -> f32 {
    10.0_f32.powf(-range_db / 20.0)
}

/// One-pole coefficient (per 48k sample) that reaches ~90% of a step in `ms`.
fn smoothing_coef(ms: f32) -> f32 {
    if ms <= 0.0 {
        1.0
    } else {
        1.0 - (-2.3 / (ms * RNNOISE_SAMPLE_RATE as f32 / 1000.0)).exp()
    }
}

fn expect_float(value: FilterValue, what: &str) -> anyhow::Result<f32> {
    match value {
        FilterValue::Float32(value) => Ok(value),
        _ => bail!("Attempted to Set {what} as non-float"),
    }
}

impl FilterHandler for NoiseSuppressionFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![
            self.get_property(NoiseSuppressionProperties::Enabled as u32),
            self.get_property(NoiseSuppressionProperties::MaxAttenuation as u32),
            self.get_property(NoiseSuppressionProperties::VadThreshold as u32),
            self.get_property(NoiseSuppressionProperties::VadGracePeriod as u32),
            self.get_property(NoiseSuppressionProperties::RetroactiveVadGracePeriod as u32),
            self.get_property(NoiseSuppressionProperties::GateRange as u32),
            self.get_property(NoiseSuppressionProperties::GateAttack as u32),
            self.get_property(NoiseSuppressionProperties::GateRelease as u32),
            self.get_property(NoiseSuppressionProperties::Mix as u32),
            self.get_property(NoiseSuppressionProperties::SideReduction as u32),
        ]
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        let prop = NoiseSuppressionProperties::try_from(id).expect("Invalid property ID");

        match prop {
            NoiseSuppressionProperties::Enabled => FilterProperty {
                id: NoiseSuppressionProperties::Enabled as u32,
                name: "Enabled".to_string(),
                symbol: "enabled".to_string(),
                value: FilterValue::Bool(self.enabled),

                min: 0.0,
                max: 1.0,

                is_input: false,
                enum_def: None,
            },

            NoiseSuppressionProperties::MaxAttenuation => FilterProperty {
                id: NoiseSuppressionProperties::MaxAttenuation as u32,
                name: "Max Attenuation".to_string(),
                symbol: "max_attenuation".to_string(),
                value: FilterValue::Float32(self.max_attenuation),

                min: 0.0,
                max: 60.0,

                is_input: false,
                enum_def: None,
            },

            NoiseSuppressionProperties::VadThreshold => FilterProperty {
                id: NoiseSuppressionProperties::VadThreshold as u32,
                name: "VAD Threshold".to_string(),
                symbol: "vad_threshold".to_string(),
                value: FilterValue::Float32(self.vad_threshold),

                min: 0.0,
                max: 1.0,

                is_input: false,
                enum_def: None,
            },

            NoiseSuppressionProperties::VadGracePeriod => FilterProperty {
                id: NoiseSuppressionProperties::VadGracePeriod as u32,
                name: "VAD Grace Period".to_string(),
                symbol: "vad_grace_period".to_string(),
                value: FilterValue::Float32(self.vad_grace_period),

                min: 0.0,
                max: 1000.0,

                is_input: false,
                enum_def: None,
            },

            NoiseSuppressionProperties::RetroactiveVadGracePeriod => FilterProperty {
                id: NoiseSuppressionProperties::RetroactiveVadGracePeriod as u32,
                name: "Retroactive VAD Grace Period".to_string(),
                symbol: "retroactive_vad_grace_period".to_string(),
                value: FilterValue::Float32(self.retroactive_vad_grace_period),

                min: 0.0,
                max: 1000.0,

                is_input: false,
                enum_def: None,
            },

            NoiseSuppressionProperties::GateRange => FilterProperty {
                id: NoiseSuppressionProperties::GateRange as u32,
                name: "Gate Range".to_string(),
                symbol: "gate_range".to_string(),
                value: FilterValue::Float32(self.gate_range),

                min: 0.0,
                max: 80.0,

                is_input: false,
                enum_def: None,
            },

            NoiseSuppressionProperties::GateAttack => FilterProperty {
                id: NoiseSuppressionProperties::GateAttack as u32,
                name: "Gate Attack".to_string(),
                symbol: "gate_attack".to_string(),
                value: FilterValue::Float32(self.gate_attack),

                min: 0.0,
                max: 500.0,

                is_input: false,
                enum_def: None,
            },

            NoiseSuppressionProperties::GateRelease => FilterProperty {
                id: NoiseSuppressionProperties::GateRelease as u32,
                name: "Gate Release".to_string(),
                symbol: "gate_release".to_string(),
                value: FilterValue::Float32(self.gate_release),

                min: 0.0,
                max: 500.0,

                is_input: false,
                enum_def: None,
            },

            NoiseSuppressionProperties::SideReduction => FilterProperty {
                id: NoiseSuppressionProperties::SideReduction as u32,
                name: "Side Reduction".to_string(),
                symbol: "side_reduction".to_string(),
                value: FilterValue::Float32(self.side_reduction),

                min: 0.0,
                max: 1.0,

                is_input: false,
                enum_def: None,
            },

            NoiseSuppressionProperties::Mix => FilterProperty {
                id: NoiseSuppressionProperties::Mix as u32,
                name: "Mix".to_string(),
                symbol: "mix".to_string(),
                value: FilterValue::Float32(self.mix),

                min: 0.0,
                max: 1.0,

                is_input: false,
                enum_def: None,
            },
        }
    }

    fn set_property(&mut self, id: u32, value: FilterValue) -> anyhow::Result<String> {
        let prop = NoiseSuppressionProperties::try_from(id).map_err(|e| anyhow!(e))?;

        match prop {
            NoiseSuppressionProperties::Enabled => {
                if let FilterValue::Bool(value) = value {
                    self.enabled = value;
                    Ok("enabled".into())
                } else {
                    bail!("Attempted to Set Enabled as non-boolean");
                }
            }

            NoiseSuppressionProperties::MaxAttenuation => {
                let value = expect_float(value, "Max Attenuation")?;
                self.max_attenuation = clamp_param(value, 0.0, MAX_ATTENUATION_DB);
                self.wet_target = wet_target_for(self.max_attenuation, self.mix);
                Ok("max_attenuation".into())
            }

            NoiseSuppressionProperties::VadThreshold => {
                let value = expect_float(value, "VAD Threshold")?;
                self.vad_threshold = clamp_param(value, 0.0, 1.0);
                self.last_speech = None;
                Ok("vad_threshold".into())
            }

            NoiseSuppressionProperties::VadGracePeriod => {
                let value = expect_float(value, "VAD Grace Period")?;
                self.vad_grace_period = clamp_param(value, 0.0, MAX_VAD_PERIOD_MS);
                self.grace_frames = ms_to_frames(self.vad_grace_period) as i64;
                Ok("vad_grace_period".into())
            }

            NoiseSuppressionProperties::RetroactiveVadGracePeriod => {
                let value = expect_float(value, "Retroactive VAD Grace Period")?;
                self.retroactive_vad_grace_period = clamp_param(value, 0.0, MAX_VAD_PERIOD_MS);
                self.retro_frames =
                    ms_to_frames(self.retroactive_vad_grace_period).min(MAX_RETRO_FRAMES);
                self.last_speech = None;
                Ok("retroactive_vad_grace_period".into())
            }

            NoiseSuppressionProperties::GateRange => {
                let value = expect_float(value, "Gate Range")?;
                self.gate_range = clamp_param(value, 0.0, MAX_GATE_RANGE_DB);
                self.gate_floor = gate_floor_for(self.gate_range);
                Ok("gate_range".into())
            }

            NoiseSuppressionProperties::GateAttack => {
                let value = expect_float(value, "Gate Attack")?;
                self.gate_attack = clamp_param(value, 0.0, MAX_GATE_TIME_MS);
                self.attack_coef = smoothing_coef(self.gate_attack);
                Ok("gate_attack".into())
            }

            NoiseSuppressionProperties::GateRelease => {
                let value = expect_float(value, "Gate Release")?;
                self.gate_release = clamp_param(value, 0.0, MAX_GATE_TIME_MS);
                self.release_coef = smoothing_coef(self.gate_release);
                Ok("gate_release".into())
            }

            NoiseSuppressionProperties::SideReduction => {
                let value = expect_float(value, "Side Reduction")?;
                self.side_reduction = clamp_param(value, 0.0, 1.0);
                Ok("side_reduction".into())
            }

            NoiseSuppressionProperties::Mix => {
                let value = expect_float(value, "Mix")?;
                self.mix = clamp_param(value, 0.0, 1.0);
                self.wet_target = wet_target_for(self.max_attenuation, self.mix);
                Ok("mix".into())
            }
        }
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, mut outputs: Vec<&mut [f32]>) {
        // This filter naturally introduces latency to the chain, so we need to always run it to
        // ensure that latency is persisted.

        self.process(&inputs);
        self.drain_output(&mut outputs);
    }
}

pub fn filter_rnnoise(
    id: Ulid,
    name: String,
    defaults: HashMap<String, FilterValue>,
    sample_rate: u32,
    channels: usize,
) -> Result<(String, FilterProperties), FilterState> {
    let filter_name = "Noise Suppression".to_string();
    let filter_desc = name.to_lowercase().replace(" ", "-");

    debug!("Filter Name: {}", filter_name);

    let callback = NoiseSuppressionFilter::new(defaults, sample_rate, channels);

    let props = FilterProperties {
        filter_id: id,
        filter_name: filter_name.to_string(),
        filter_nick: filter_name.to_string(),
        filter_description: format!("{}/{}", APP_NAME_ID, filter_desc),

        class: MediaClass::Duplex,
        app_id: APP_ID.to_string(),
        app_name: APP_NAME.to_string(),
        linger: false,

        callback: Box::new(callback),
        ready_sender: None,
    };

    Ok((filter_name, props))
}

#[derive(Debug, PartialEq)]
#[repr(u32)]
enum NoiseSuppressionProperties {
    Enabled = 0,
    MaxAttenuation = 1,
    VadThreshold = 2,
    VadGracePeriod = 3,
    RetroactiveVadGracePeriod = 4,
    Mix = 5,
    GateRange = 6,
    GateAttack = 7,
    GateRelease = 8,
    SideReduction = 9,
}

impl FromStr for NoiseSuppressionProperties {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Enabled" => Ok(NoiseSuppressionProperties::Enabled),
            "MaxAttenuation" => Ok(NoiseSuppressionProperties::MaxAttenuation),
            "VadThreshold" => Ok(NoiseSuppressionProperties::VadThreshold),
            "VadGracePeriod" => Ok(NoiseSuppressionProperties::VadGracePeriod),
            "RetroactiveVadGracePeriod" => {
                Ok(NoiseSuppressionProperties::RetroactiveVadGracePeriod)
            }
            "Mix" => Ok(NoiseSuppressionProperties::Mix),
            "GateRange" => Ok(NoiseSuppressionProperties::GateRange),
            "GateAttack" => Ok(NoiseSuppressionProperties::GateAttack),
            "GateRelease" => Ok(NoiseSuppressionProperties::GateRelease),
            "SideReduction" => Ok(NoiseSuppressionProperties::SideReduction),
            _ => Err(format!("Unknown variant: {s}")),
        }
    }
}

impl TryFrom<u32> for NoiseSuppressionProperties {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(NoiseSuppressionProperties::Enabled),
            1 => Ok(NoiseSuppressionProperties::MaxAttenuation),
            2 => Ok(NoiseSuppressionProperties::VadThreshold),
            3 => Ok(NoiseSuppressionProperties::VadGracePeriod),
            4 => Ok(NoiseSuppressionProperties::RetroactiveVadGracePeriod),
            5 => Ok(NoiseSuppressionProperties::Mix),
            6 => Ok(NoiseSuppressionProperties::GateRange),
            7 => Ok(NoiseSuppressionProperties::GateAttack),
            8 => Ok(NoiseSuppressionProperties::GateRelease),
            9 => Ok(NoiseSuppressionProperties::SideReduction),
            _ => Err(format!("Invalid Value: {value}")),
        }
    }
}
