# RF-EPiano 0.1.0 verification

Verified on Windows with the RackForge 0.1.25 checkout at
`7a14c68aa208290587d985d606b0dbc17600ee07`, Rust 1.98.0 MSVC and the
unmodified OpenWurli 0.9.1 reference at `c05403b9b60f95b4908ae19727682e81803a7351`.

- `rackforge-store pack-wasm` produced the optimized portable component.
- `rackforge-store install-local` accepted the final archive in a separate
  test store. The normal RackForge library was not used for verification.
- The installed component was instantiated by the RackForge portable
  runtime. Notes, sustain, release, and all three controls were compared
  against OpenWurli at exact sample offsets. The test uses 224 blocks of
  257 frames per rate and checks both output channels.
- Maximum absolute sample difference: `6.0e-8` at 44.1 kHz, `3.7e-8` at
  48 kHz, and `4.5e-8` at 96 kHz. Outputs were finite; the comparison is
  numerical equivalence within tolerance, rather than bit equality.
- State round-trip, rejection of a non-finite value without changing state,
  factory defaults, and repeated reset were exercised through WASM.
- A native allocation-instrumented test found no allocations, reallocations
  or frees when rendering notes, operating sustain, or resetting. A held
  note was silenced by reset and the instrument played again afterward.
- The installed PLAY page passed the Chromium sandboxed-iframe fixture:
  initial values, keyboard editing of each control, canonical host updates,
  rejection rollback, context reload and responsive day/stage layout.

`rackforge-core stress` at 48 kHz, 128 frames and 192 blocks on this machine:

| Voices requested | Peak absolute sample | Mean render time / block period |
| ---: | ---: | ---: |
| 8 | 0.343280 | 0.105 |
| 32 | 0.828740 | 0.158 |
| 64 | 1.058299 | 0.235 |

The stress run is an offline diagnostic, not a hard real-time or platform
qualification. The 64-note stress reached above digital full scale with
the original +14 dB alignment; the adapter adds no limiter. The measurement
does not establish performance on Raspberry Pi, Android, or browser audio.

Final `RF-EPiano-0.1.0.rfplugin` SHA-256:
`c8e1f656073f8cfe66b9a42c0554fe9a76733d83895cd52b8495a0ec9a1678fa`.
