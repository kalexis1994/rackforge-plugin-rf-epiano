//! RF-EPiano: OpenWurli's 200A engine behind the portable RackForge contract.
//! The adapter offers a second reed voicing and retains OpenWurli's +14 dB
//! output alignment, duplicated from mono to stereo.

use openwurli_dsp::engine::{PreparedResetState, WurliEngine};
use rackforge_plugin_sdk::{MidiEvent, ParameterEvent, Processor, export_processor};

pub const MAX_FRAMES: usize = 4096;
pub const OUTPUT_ALIGNMENT: f32 = 5.011_872_3;
const DEFAULTS: [f64; 3] = [0.8, 0.5, 0.0];
const BASE_STATE_BYTES: usize = 28;
const BARK_STATE_TAG: u8 = 1;
const STATE_MAGIC: &[u8; 4] = b"RFE1";

/// Values are the upstream engine's normalized controls, never post-output gain.
pub struct EPiano {
    parameters: [f64; 3],
    bark: bool,
    engine: Option<WurliEngine>,
    reset_state: Option<PreparedResetState>,
    mono: Vec<f32>,
    maximum_frames: u32,
}

impl Default for EPiano {
    fn default() -> Self {
        Self {
            parameters: DEFAULTS,
            bark: false,
            engine: None,
            reset_state: None,
            mono: Vec::new(),
            maximum_frames: 0,
        }
    }
}

impl EPiano {
    fn sync_parameters(&mut self) {
        if let Some(engine) = &mut self.engine {
            engine.set_volume(self.parameters[0]);
            engine.set_tremolo_depth(self.parameters[1]);
            engine.set_speaker_character(self.parameters[2]);
            engine.set_reed_swing_scale(if self.bark { 1.5 } else { 1.0 });
            // Preserve the upstream default sound: experimental MLP is disabled.
            engine.set_mlp_enabled(false);
        }
    }

    fn apply_midi(&mut self, event: &MidiEvent) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        if event.length != 3 || event.data[1] > 127 || event.data[2] > 127 {
            return;
        }
        let [status, note, value] = event.data;
        match status & 0xf0 {
            0x90 if value > 0 => engine.note_on(note, f32::from(value) / 127.0),
            0x80 | 0x90 => engine.note_off(note),
            0xb0 => match note {
                64 => engine.set_sustain(value >= 64),
                120 => engine.all_sound_off(),
                121 => engine.set_sustain(false),
                123..=127 => engine.release_all_keys(),
                _ => (),
            },
            _ => (),
        }
    }

    fn render_segment(&mut self, output: &mut [f32], start: usize, end: usize) {
        if start == end {
            return;
        }
        let Some(engine) = &mut self.engine else {
            return;
        };
        let mono = &mut self.mono[..end - start];
        engine.render(mono);
        for (stereo, sample) in output[start * 2..end * 2].chunks_exact_mut(2).zip(mono) {
            let value = *sample * OUTPUT_ALIGNMENT;
            stereo.fill(value);
        }
    }
}

impl Processor for EPiano {
    fn prepare(
        &mut self,
        sample_rate: f64,
        maximum_frames: u32,
        inputs: u32,
        outputs: u32,
    ) -> bool {
        if !sample_rate.is_finite()
            || !(8000.0..=192000.0).contains(&sample_rate)
            || maximum_frames == 0
            || maximum_frames as usize > MAX_FRAMES
            || inputs != 0
            || outputs != 2
        {
            return false;
        }
        // Construction and settling run only on the host's preparation thread.
        let mut engine = WurliEngine::new(sample_rate);
        engine.warm_up();
        engine.ensure_buffer_capacity(maximum_frames as usize);
        let reset_state = engine.prepared_reset_state();
        self.mono = vec![0.0; maximum_frames as usize];
        self.engine = Some(engine);
        self.reset_state = Some(reset_state);
        self.maximum_frames = maximum_frames;
        self.sync_parameters();
        true
    }

    fn set_parameter(&mut self, index: u32, value: f64) -> bool {
        let Some(parameter) = self.parameters.get_mut(index as usize) else {
            return false;
        };
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return false;
        }
        *parameter = value;
        self.sync_parameters();
        true
    }

    fn get_parameter(&self, index: u32) -> Option<f64> {
        self.parameters.get(index as usize).copied()
    }

    fn reset(&mut self) {
        if let (Some(engine), Some(state)) = (&mut self.engine, &self.reset_state) {
            engine.reset_to_prepared(state);
        }
        self.mono.fill(0.0);
    }

    fn load_preset(&mut self, id: &str) -> bool {
        let (parameters, bark) = match id {
            "default" => (DEFAULTS, false),
            "direct" => ([0.8, 0.0, 0.0], false),
            "cabinet" => ([0.8, 0.0, 1.0], false),
            "tremolo" => ([0.8, 0.65, 1.0], false),
            "bark" => ([0.7, 0.0, 0.0], true),
            _ => return false,
        };
        self.parameters = parameters;
        self.bark = bark;
        self.sync_parameters();
        true
    }

    fn save_state(&self, destination: &mut [u8]) -> Option<usize> {
        // Keep the original layout for all standard programs.
        let length = BASE_STATE_BYTES + usize::from(self.bark);
        let bytes = destination.get_mut(..length)?;
        bytes[..4].copy_from_slice(STATE_MAGIC);
        for (slot, value) in bytes[4..BASE_STATE_BYTES]
            .chunks_exact_mut(8)
            .zip(self.parameters)
        {
            slot.copy_from_slice(&value.to_le_bytes());
        }
        if self.bark {
            bytes[BASE_STATE_BYTES] = BARK_STATE_TAG;
        }
        Some(length)
    }

    fn load_state(&mut self, bytes: &[u8]) -> bool {
        let Some((base, suffix)) = bytes.split_at_checked(BASE_STATE_BYTES) else {
            return false;
        };
        if &base[..4] != STATE_MAGIC {
            return false;
        }
        let bark = match suffix {
            [] => false,
            [BARK_STATE_TAG] => true,
            _ => return false,
        };
        let mut values = [0.0; 3];
        for (value, source) in values.iter_mut().zip(base[4..].chunks_exact(8)) {
            *value = f64::from_le_bytes(source.try_into().expect("eight-byte chunk"));
            if !value.is_finite() || !(0.0..=1.0).contains(value) {
                return false;
            }
        }
        self.parameters = values;
        self.bark = bark;
        self.sync_parameters();
        true
    }

    fn process(
        &mut self,
        _input: &[f32],
        output: &mut [f32],
        midi: &[MidiEvent],
        parameters: &[ParameterEvent],
        frames: u32,
        inputs: u32,
        outputs: u32,
    ) {
        output.fill(0.0);
        if inputs != 0
            || outputs != 2
            || frames > self.maximum_frames
            || output.len() != frames as usize * 2
            || self.engine.is_none()
        {
            return;
        }
        let mut cursor = 0;
        let mut mi = 0;
        let mut pi = 0;
        while cursor < frames as usize {
            // Host events are ordered. Parameters precede notes at equal offsets.
            while pi < parameters.len() && parameters[pi].frame as usize <= cursor {
                let event = parameters[pi];
                let _ = self.set_parameter(event.index, event.value);
                pi += 1;
            }
            while mi < midi.len() && midi[mi].frame as usize <= cursor {
                self.apply_midi(&midi[mi]);
                mi += 1;
            }
            let end = (frames as usize)
                .min(midi.get(mi).map_or(frames as usize, |e| e.frame as usize))
                .min(
                    parameters
                        .get(pi)
                        .map_or(frames as usize, |e| e.frame as usize),
                );
            self.render_segment(output, cursor, end);
            cursor = end;
        }
    }
}

export_processor!(
    EPiano,
    max_frames = 4096,
    max_input_channels = 0,
    max_output_channels = 2,
    max_midi_events = 512,
    max_parameter_events = 512,
    max_transfer_bytes = 4096
);
