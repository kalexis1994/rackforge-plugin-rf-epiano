use std::{env, fs, path::PathBuf};

use openwurli_reference::WurliEngine;
use rackforge_plugin_runtime::{MidiEvent, ParameterEvent, PortableEngine, RuntimeLimits};

#[derive(Clone, Copy)]
enum Cue {
    On(u8, u8),
    Off(u8),
    Pedal(bool),
    Parameter(u32, f64),
}

#[test]
fn package_component_preserves_openwurli_sound_and_state() {
    let path = env::var_os("RF_EPIANO_COMPONENT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../target/wasm32-unknown-unknown/release/rackforge_rf_epiano.wasm")
        });
    let engine = PortableEngine::new(RuntimeLimits::default()).unwrap();
    let module = engine
        .compile(&fs::read(&path).unwrap_or_else(|error| {
            panic!(
                "build the WASM component first ({}: {error})",
                path.display()
            );
        }))
        .expect("RackForge accepts the import-free component");
    let mut instance = module.instantiate().unwrap();

    for (id, values) in [
        ("direct", [0.8, 0.0, 0.0]),
        ("cabinet", [0.8, 0.0, 1.0]),
        ("tremolo", [0.8, 0.65, 1.0]),
    ] {
        instance.load_preset(id).unwrap();
        for (index, value) in values.into_iter().enumerate() {
            assert_eq!(instance.get_parameter(index as u32).unwrap(), value, "{id}");
        }
    }
    let state = instance.save_state().unwrap();
    assert!(instance.load_preset("unknown").is_err());
    assert_eq!(instance.save_state().unwrap(), state);

    for rate in [44_100.0, 48_000.0, 96_000.0] {
        const FRAMES: usize = 257;
        const BLOCKS: usize = 224;
        instance.load_preset("default").unwrap();
        instance.prepare(rate, FRAMES as u32, 0, 2).unwrap();
        let mut original = WurliEngine::new(rate);
        original.warm_up();

        let cues = [
            (37, Cue::On(60, 45)),
            (FRAMES * 6 + 4, Cue::Parameter(0, 0.7)),
            (FRAMES * 11 + 111, Cue::Parameter(1, 0.83)),
            (FRAMES * 11 + 111, Cue::On(67, 104)),
            (FRAMES * 21 + 247, Cue::Pedal(true)),
            (FRAMES * 30 + 9, Cue::Off(60)),
            (FRAMES * 35 + 126, Cue::Off(67)),
            (FRAMES * 48 + 1, Cue::Parameter(2, 1.0)),
            (FRAMES * 63 + 213, Cue::On(48, 119)),
            (FRAMES * 77 + 84, Cue::Off(48)),
            (FRAMES * 95 + 67, Cue::Pedal(false)),
            (FRAMES * 110 + 255, Cue::On(76, 81)),
            (FRAMES * 140 + 3, Cue::Off(76)),
        ];
        let mut output = [0.0_f32; FRAMES * 2];
        let mut mono = [0.0_f32; FRAMES];
        let mut peak = 0.0_f32;
        let mut max_difference = 0.0_f32;
        // Match the upstream wrapper's +14 dB output alignment.
        let align = 10.0_f32.powf(14.0_f32 * 0.05_f32);
        for block in 0..BLOCKS {
            let start = block * FRAMES;
            let mut midi = Vec::new();
            let mut parameters = Vec::new();
            let mut cursor = 0;
            for &(frame, cue) in cues
                .iter()
                .filter(|(frame, _)| (start..start + FRAMES).contains(frame))
            {
                let offset = frame - start;
                original.render(&mut mono[cursor..offset]);
                cursor = offset;
                match cue {
                    Cue::On(note, velocity) => {
                        midi.push(MidiEvent::new(offset as u32, &[0x90, note, velocity]).unwrap());
                        original.note_on(note, f32::from(velocity) / 127.0);
                    }
                    Cue::Off(note) => {
                        midi.push(MidiEvent::new(offset as u32, &[0x90, note, 0]).unwrap());
                        original.note_off(note);
                    }
                    Cue::Pedal(held) => {
                        midi.push(
                            MidiEvent::new(offset as u32, &[0xb0, 64, if held { 127 } else { 0 }])
                                .unwrap(),
                        );
                        original.set_sustain(held);
                    }
                    Cue::Parameter(index, value) => {
                        parameters.push(ParameterEvent {
                            frame: offset as u32,
                            index,
                            value,
                        });
                        match index {
                            0 => original.set_volume(value),
                            1 => original.set_tremolo_depth(value),
                            2 => original.set_speaker_character(value),
                            _ => unreachable!(),
                        }
                    }
                }
            }
            original.render(&mut mono[cursor..]);
            instance
                .process_interleaved_with_events(
                    &[],
                    &mut output,
                    FRAMES as u32,
                    &midi,
                    &parameters,
                )
                .unwrap();
            for (stereo, reference) in output.chunks_exact(2).zip(mono) {
                assert!(stereo[0].is_finite());
                assert_eq!(stereo[0], stereo[1]);
                peak = peak.max(stereo[0].abs());
                max_difference = max_difference.max((stereo[0] - reference * align).abs());
            }
            if block == 0 {
                assert!(output[..37 * 2].iter().all(|value| value.abs() < 1e-5));
            }
        }
        assert!(peak > 0.05, "the comparison must include sounding notes");
        assert!(
            max_difference < 2e-6,
            "{rate} Hz: max difference {max_difference}"
        );
        eprintln!(
            "RF-EPIANO_REFERENCE rate={rate:.0} peak={peak:.6} max_abs_difference={max_difference:.9}"
        );

        assert_eq!(instance.get_parameter(0).unwrap(), 0.7);
        assert_eq!(instance.get_parameter(1).unwrap(), 0.83);
        assert_eq!(instance.get_parameter(2).unwrap(), 1.0);
        let state = instance.save_state().unwrap();
        instance.load_preset("default").unwrap();
        assert_eq!(instance.get_parameter(0).unwrap(), 0.8);
        assert_eq!(instance.get_parameter(1).unwrap(), 0.5);
        assert_eq!(instance.get_parameter(2).unwrap(), 0.0);
        instance.load_state(&state).unwrap();
        assert_eq!(instance.save_state().unwrap(), state);
        let mut invalid = state.clone();
        invalid[20..28].copy_from_slice(&f64::NAN.to_le_bytes());
        assert!(instance.load_state(&invalid).is_err());
        assert_eq!(instance.save_state().unwrap(), state);

        // Repeated resets must retain parameters and immediately clear the tail.
        instance.reset().unwrap();
        instance.reset().unwrap();
        instance
            .process_interleaved(&[], &mut output, FRAMES as u32)
            .unwrap();
        assert!(output.iter().all(|value| value.abs() < 1e-5));
        assert_eq!(instance.save_state().unwrap(), state);
    }

    // The legacy payload has exactly the bytes emitted by 0.1.0 and 0.1.1.
    let mut legacy = b"RFE1".to_vec();
    for value in [0.8_f64, 0.0, 0.0] {
        legacy.extend(value.to_le_bytes());
    }
    let strike = |instance: &mut rackforge_plugin_runtime::PortableInstance| {
        instance.reset().unwrap();
        let mut audio = [0.0_f32; 512];
        let note = MidiEvent::new(0, &[0x90, 45, 112]).unwrap();
        instance
            .process_interleaved_with_events(&[], &mut audio, 256, &[note], &[])
            .unwrap();
        assert!(audio.iter().all(|value| value.is_finite()));
        audio
    };

    let mut bark = module.instantiate().unwrap();
    bark.load_preset("bark").unwrap();
    bark.prepare(48_000.0, 256, 0, 2).unwrap();
    let bark_audio = strike(&mut bark);
    let bark_state = bark.save_state().unwrap();
    assert_eq!(&bark_state[..4], b"RFE1");
    assert_eq!(&bark_state[4..12], &0.7_f64.to_le_bytes());
    assert_eq!(&bark_state[12..legacy.len()], &legacy[12..]);
    assert_eq!(&bark_state[legacy.len()..], &[1]);
    assert!(bark_audio.iter().any(|sample| sample.abs() > 0.01));

    let mut restored = module.instantiate().unwrap();
    restored.load_state(&bark_state).unwrap();
    restored.prepare(48_000.0, 256, 0, 2).unwrap();
    assert_eq!(strike(&mut restored), bark_audio);

    let mut invalid = bark_state;
    *invalid.last_mut().unwrap() = 2;
    assert!(restored.load_state(&invalid).is_err());
    assert_eq!(strike(&mut restored), bark_audio);

    restored.load_state(&legacy).unwrap();
    assert_eq!(restored.save_state().unwrap(), legacy);
    let direct_audio = strike(&mut restored);
    assert_ne!(direct_audio, bark_audio);
    bark.load_preset("direct").unwrap();
    assert_eq!(strike(&mut bark), direct_audio);
}
