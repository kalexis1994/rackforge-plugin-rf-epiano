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

The lookup tables, equations and default solver configuration are unchanged.
The RackForge adapter uses the prepared reset for real-time operation.
