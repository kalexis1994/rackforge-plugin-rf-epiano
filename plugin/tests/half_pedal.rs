use rackforge_plugin_sdk::{MidiEvent, Processor};
use rackforge_rf_epiano::EPiano;

fn render(pedal: u8, held: bool) -> Vec<f32> {
    let mut plugin = EPiano::default();
    assert!(plugin.prepare(48_000.0, 256, 0, 2));
    assert!(plugin.load_preset("direct"));
    let mut output = Vec::new();
    let mut block = [0.0; 512];
    for index in 0..96 {
        let events = match index {
            0 => vec![MidiEvent::new(0, [0x90, 60, 90], 3).unwrap()],
            10 => vec![MidiEvent::new(17, [0xb0, 64, pedal], 3).unwrap()],
            20 if !held => vec![MidiEvent::new(31, [0x80, 60, 0], 3).unwrap()],
            _ => vec![],
        };
        plugin.process(&[], &mut block, &events, &[], 256, 0, 2);
        assert!(block.iter().all(|value| value.is_finite()));
        output.extend_from_slice(&block);
    }
    output
}

#[test]
fn cc64_controls_release_progressively_through_the_plugin_adapter() {
    let levels: Vec<f64> = [0, 32, 64, 96, 127]
        .into_iter()
        .map(|pedal| {
            let audio = render(pedal, false);
            audio[32_768..]
                .iter()
                .map(|value| f64::from(*value).powi(2))
                .sum()
        })
        .collect();
    assert!(
        levels.windows(2).all(|pair| pair[1] > pair[0] * 1.1),
        "{levels:?}"
    );
    assert_eq!(render(0, true), render(64, true));
}
