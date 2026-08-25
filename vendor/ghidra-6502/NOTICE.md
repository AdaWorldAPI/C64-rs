# Vendored files — provenance

`data/languages/6502.slaspec`, `6502.pspec`, `6502.cspec`, `6502.ldefs` are
copied verbatim, unmodified, from
[`AdaWorldAPI/ghidra`](https://github.com/AdaWorldAPI/ghidra) —
`Ghidra/Processors/6502/data/languages/`, itself a fork of the National
Security Agency's [`NationalSecurityAgency/ghidra`](https://github.com/NationalSecurityAgency/ghidra).

Ghidra is licensed under the Apache License, Version 2.0. The full license
text is preserved verbatim in `LICENSE-APACHE-GHIDRA`.

These files were verified small (< 9 KiB total, 4 files, no external
`@include`s) and permissively licensed before vendoring — see
`CLAUDE.md`'s GPL/licensing note. Nothing here is a hand-transcription or
re-implementation: the compiled MOS 6502 SLEIGH semantics come entirely
from these unmodified upstream files, compiled at build time via the
`sleigh-compiler` crate (a Rust binding over Ghidra's own C++ SLEIGH
compiler, also Apache-2.0). This repo never implements 6502 decode logic
itself — see `CLAUDE.md`'s "no new 6502 decoder" hard rule.
