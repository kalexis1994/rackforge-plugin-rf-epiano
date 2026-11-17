# OpenWurli DSP provenance

Upstream: [OpenWurli](https://github.com/hal0zer0/openwurli), by hal0zer0 and contributors.
Vendored from `crates/openwurli-dsp` at commit
`c05403b9b60f95b4908ae19727682e81803a7351` (0.9.1), GPL-3.0-or-later.
`LICENSE` is copied from upstream. Source of `melange-primitives` is pinned to
the same revision as upstream, `7ecb36c7`.

Local changes for RF-EPiano:

- Make the DSP manifest standalone while retaining its default features.
- Add a `rackforge-reset` feature. Capture the settled silent circuit during
  preparation and copy that state when RackForge asks for a real-time reset.
  Retain parameter targets and reuse the already allocated buffers.
- Add clone support to the fixed-size state used in that snapshot and a
  bounded history copy for the oversampler.
- Expose the existing reed swing multiplier to the RackForge adapter. The
  200A Bark program sets it to 1.5 at note-on. The default remains 1.0.

- Add continuous sustain travel, including half-pedal felt pressure and
  recapturing released reeds. Pedal movements retain the reed envelopes and
  the existing register-dependent damper landing time. Forced releases and
  re-struck older voices finish damping independently of pedal movements.

The RackForge adapter uses the prepared reset for real-time operation. Its
continuous pedal extends the upstream damper model with a provisional pressure
curve. The existing synthesis tables and circuit solver configuration are used.
