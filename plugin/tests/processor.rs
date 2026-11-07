use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use rackforge_plugin_sdk::{MidiEvent, Processor};
use rackforge_rf_epiano::EPiano;

struct TrackedAllocator;
thread_local! { static MEMORY_OPERATIONS: Cell<Option<usize>> = const { Cell::new(None) }; }

fn count_memory_operation() {
    let _ = MEMORY_OPERATIONS.try_with(|operations| {
        if let Some(count) = operations.get() {
            operations.set(Some(count + 1));
        }
    });
}

unsafe impl GlobalAlloc for TrackedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_memory_operation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_memory_operation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_memory_operation();
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count_memory_operation();
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: TrackedAllocator = TrackedAllocator;

#[test]
fn prepared_reset_silences_sustained_notes_without_memory_operations() {
    let mut plugin = EPiano::default();
    assert!(plugin.prepare(48_000.0, 256, 0, 2));
    assert!(plugin.set_parameter(2, 1.0));
    let note_on = MidiEvent::new(19, [0x90, 60, 112], 3).unwrap();
    let pedal = MidiEvent::new(0, [0xb0, 64, 127], 3).unwrap();
    let note_off = MidiEvent::new(41, [0x90, 60, 0], 3).unwrap();
    let mut audio = [0.0_f32; 512];

    MEMORY_OPERATIONS.with(|count| count.set(Some(0)));
    plugin.process(&[], &mut audio, &[note_on], &[], 256, 0, 2);
    plugin.process(&[], &mut audio, &[pedal, note_off], &[], 256, 0, 2);
    let held_peak = audio
        .iter()
        .fold(0.0_f32, |peak, value| peak.max(value.abs()));
    plugin.reset();
    plugin.reset();
    plugin.process(&[], &mut audio, &[], &[], 256, 0, 2);
    let reset_peak = audio
        .iter()
        .fold(0.0_f32, |peak, value| peak.max(value.abs()));
    plugin.process(&[], &mut audio, &[note_on], &[], 256, 0, 2);
    let replay_peak = audio
        .iter()
        .fold(0.0_f32, |peak, value| peak.max(value.abs()));
    let operations = MEMORY_OPERATIONS.with(|count| count.take().unwrap());

    assert_eq!(operations, 0);
    assert!(held_peak > 0.01, "the pedal should hold a sounding note");
    assert!(reset_peak < 1e-5, "reset left a tail at {reset_peak}");
    assert!(replay_peak > 0.01, "the instrument should play after reset");
    assert_eq!(plugin.get_parameter(2), Some(1.0));
}
