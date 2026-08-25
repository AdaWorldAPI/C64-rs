# CLAUDE.md — C64-rs

> Read first, every session. This repo is the falsifier for a specific
> architectural claim, not a C64 emulator project. Read
> `.claude/plans/c64-falsifier-v1.md` before adding scope.

## What this is

A bounded proof, not a product: can a C64 `.PRG` be loaded into a flat
64 KiB address space, lifted through Ghidra SLEIGH → P-code → R2IL
(`AdaWorldAPI/r2sleigh`), and exposed through a Java/Panama mask-native
surface (`AdaWorldAPI/lance-graph-java`) **without materializing the 64K
memory or the instruction population as Java objects**? See the mission
brief preserved in `.claude/plans/c64-falsifier-v1.md` for the full
ESTABLISHED / MISSING / GAP / PROBE / ORACLE / KILL-CONDITIONS report this
repo exists to answer.

## Hard rules (carried over from the falsifier report)

- **No new 6502 decoder.** Ghidra already has a complete, unmodified
  `Ghidra/Processors/6502/` SLEIGH module (verified: flat 16-bit/64KiB
  space, no banking, LDA/STA/AND/ADC/JMP/JSR/BEQ/BNE/RTS all present).
  Consume it, never re-implement it.
- **No new C64-specific IR.** `r2sleigh`'s P-code→R2IL translation
  (`r2sleigh-lift::Disassembler::from_sla`) is already architecture-agnostic
  — one generic opcode table, not a per-arch lifter. 6502 wiring into it is
  mechanical (a CLI match-arm + a compiled `.sla`), not a code-path gap.
- **No Java object population for the 64K address space or the
  instruction population.** Everything routes through `AddressMask`
  (`crates/c64-core/src/mask.rs`) — a packed bitset, never a `Vec<u16>` —
  except through a method explicitly named `materialize_*`, mirroring
  `lance-graph-java`'s `materializeRows()` convention and its reflective
  allowlist enforcement (`GraphHopTest`). Port that enforcement test here
  before claiming the no-materialization property, don't just assert it.
- **No per-byte or per-instruction FFM/Panama crossing**, when the Panama
  probe lands. Crossings are bulk (∝ population, one call) or lifecycle
  (open/close), per `lance-graph-java/docs/abi.md` §6 — the anti-JNI rule.
- **JSR/LDA/$A9 etc. are never their own `classid`.** If an OGAR
  `ClassView`/`ogar-loco` seam is built later, `classid` says "these
  resident bytes have a C64-machine/instruction/function reading," never
  "this byte is opcode $A9." The opcode stays resident/decode-derived
  content, read through the classid's ClassView — not encoded in it.
- **Real corrected finding (2026-08-24 session):** hosting R2IL's
  Call/Branch/register semantics under OGAR's existing
  `ClassArm::Functions` would mean widening what "Functions" means today —
  it currently resolves to `ActionDef`/`KausalSpec` (a workflow/state-machine
  handler shape: predicate/subject/temporal/modal/guard/RBAC — no opcode,
  operand, branch target, or flags field). The closer-fitting existing OGAR
  carrier for faithful low-level executable semantics is `ogar-loco`'s
  `Call`/`FunctionBody`/`Vocabulary` call-ABI. `ogar-vocab`'s
  `ConceptDomain::BinaryLifting` (`0xC4XX`) is the pre-reserved, currently
  -empty classid home for this domain, and its own doc comment names Ghidra
  and r2sleigh explicitly as anticipated dual consumers. Don't re-derive
  this — read the plan doc's Probe C section first.
  **Widened 2026-08-25 (operator-directed):** platform HARDWARE concepts
  now have their own domain — `ConceptDomain::Mmio` (`0xC6XX`, "one hex
  digit short of the C64", OGAR PR #284): `mmio_chip` / `mmio_register` /
  `rom_image` / `machine_memory_map`, container KINDS only. `0xC4XX` stays
  the architecture-agnostic lift/IR home; a `$D021` register is a platform
  fact, not an IR concept. The opcode rule above is UNCHANGED by the mint —
  registers and mnemonics are content rows read through the ClassView.
- **GPL/licensing boundary (fence widened 2026-08-25, operator-approved).**
  zinc64 (GPLv3), Frodo (GPLv2), CrabSID, resid-rs may be read for
  behavioral/test-vector reference (hardware register semantics, timing)
  but their code is never transcribed into this repo or into `ogar-vocab`.
  Lawful sources for minted classid concepts: Ghidra's Apache-2.0 SLEIGH
  specs, **permissively-licensed implementations (`AdaWorldAPI/rust64` is
  MIT — its `vic.rs`/`sid.rs`/`cia.rs` are transcribable hardware
  reference, which SLEIGH is not: SLEIGH gives opcodes, it says nothing
  about `$D021`)**, and vendor datasheets (6526/6567/6581 register facts).
  The original fence named only SLEIGH because the assumed hardware
  references were GPL emulators; rust64 being MIT changes the set, not
  the rule.
- **Kill conditions are real exit criteria, not decoration.** See the plan
  doc. If Probe A's parity oracle diverges on a legal 6502 instruction, or
  Probe C needs per-instruction Java objects to preserve required semantics,
  say so and stop — don't quietly widen scope to route around it.

## Autonomous session policy

This repo is being built and merged autonomously overnight per explicit
operator instruction (2026-08-25 session). Standing authorization for this
repo only: open small, disjoint, test-covered PRs and merge them without
per-PR confirmation, provided each PR (a) has passing tests run locally
before push, (b) stays inside one probe's scope (A, B, or C — see the plan),
and (c) does not touch licensing-sensitive content (see GPL boundary above).
Anything crossing those lines — new classid minting in `ogar-vocab`, any
change to a sibling repo, anything that could be read as circumventing a
license — stops and waits for the operator awake, per every sibling repo's
own "ask, don't file" convention for cross-repo/licensing decisions.

## Probe status

- **Probe B (physics)**: `crates/c64-core` — flat 64 KiB `Memory`, PRG
  loader, `AddressMask` bitset, `RowStore` lane join, materialization
  enforcement test. In progress.
- **Probe A (lift parity)**: `crates/c64-lift` — vendored, unmodified
  Ghidra 6502 SLEIGH spec (`vendor/ghidra-6502/`, Apache-2.0), compiled at
  build time via `sleigh-compiler` (no Gradle/Java Ghidra build needed —
  verified), lifted through `r2sleigh-lift`'s existing generic
  `Disassembler::from_sla`. The fixture's hand-computed BNE/JSR targets are
  checked against the real lifted R2IL, verified as a real falsifier via a
  disable-run. **Not yet done**: the full Ghidra-headless-analyzer parity
  oracle the plan originally specified — see
  `.claude/plans/c64-falsifier-v1.md` Probe A for the honest gap.
- **Probe C (transcoding seam)**: not started — R2IL → `ogar-loco`
  Call/FunctionBody, classid under `0xC4XX` `BinaryLifting` (reserved, not
  yet minted — minting is an operator-gated decision, see above).

## Build

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```
