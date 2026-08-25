# C64-rs

A bounded architecture falsifier, not a C64 emulator project.

**The question**: can a C64 `.PRG` be loaded into one flat 64 KiB address
space, lifted through Ghidra SLEIGH → P-code → R2IL, and exposed through a
Java/Panama mask-native surface without materializing the 64K memory or the
instruction population as Java objects?

See [`CLAUDE.md`](./CLAUDE.md) and
[`.claude/plans/c64-falsifier-v1.md`](./.claude/plans/c64-falsifier-v1.md)
for the full report and probe sequence.

## Status

- **Probe B (physics)** — `crates/c64-core`: flat 64 KiB `Memory` + PRG
  loader + packed `AddressMask` bitset. In progress.
- **Probe A (lift parity)** — not started.
- **Probe C (transcoding seam)** — spec only, gated on operator sign-off
  for classid minting.

## Build

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```
