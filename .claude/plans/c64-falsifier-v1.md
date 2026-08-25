# C64/6502 architecture falsifier — v1

> Preserved from the 2026-08-24/25 session that scoped this repo. Read
> `../../CLAUDE.md` first; this is the detail behind its hard rules.

## The question

Can a C64 `.PRG` be loaded into one flat 64 KiB address space, lifted
through Ghidra SLEIGH → P-code → r2sleigh → R2IL, and exposed through a
Java/Panama surface **without materializing the 64K memory or the
instruction population as Java objects**?

## ESTABLISHED (verified against real source, 2026-08-24)

- **Ghidra** ships a complete, unmodified MOS 6502 SLEIGH module
  (`Ghidra/Processors/6502/data/languages/6502.slaspec`): little-endian,
  `define space RAM type=ram_space size=2 default;` (flat 16-bit/64KiB, no
  bank/segment/paging modeled anywhere), real constructors for
  LDA/STA/AND/ADC/JMP/JMP-indirect/JSR/BEQ/BNE/RTS. `.sla` is compiled from
  `.slaspec` at build time (`gradle/processorProject.gradle`), not checked
  in.
- **r2sleigh** already has a fully generic
  `Disassembler::from_sla(sla_bytes: &[u8], pspec: &str, arch_name: &str)`
  (`crates/r2sleigh-lift/src/disasm.rs:149`). P-code→R2IL is one generic
  match over Ghidra's ~50 canonical P-code opcodes
  (`disasm.rs:462-739`), driven by opcode identity, not arch name — no
  per-architecture lifter exists in the crate. 6502 is simply absent from
  the CLI's arch dispatch `match` (`r2sleigh-cli/src/main.rs:625-730`),
  which is proven incomplete/hand-maintained regardless (`mips` is a
  defined Cargo feature with no match arm either).
- **lance-graph-java**'s mask-native ABI (`native/lgj-abi`, `docs/abi.md`)
  is real and structurally enforced: 21 `extern "C"` symbols, every one
  bulk (∝ n_rows) or lifecycle, none per-element; `GraphHopTest`
  reflectively bans `long[]`/Collection fields and return types except from
  `materialize*`-named methods; an allocation-independence gate measures
  `ThreadMXBean#getThreadAllocatedBytes` to prove cost is population-size
  independent. **No generic byte-addressable-memory abstraction exists
  yet** — the only "64K" concept there is 64K graph *rows*, not bytes.
- **OGAR**'s `ClassArm{View, Functions}` is real shipped code
  (`lance-graph-contract/src/facet.rs:566-583`), but `Functions` is
  documented/consumed everywhere as reaching `ActionDef`/`KausalSpec` — a
  workflow/state-machine handler shape with no opcode/operand/branch-target
  field. The closer-fitting carrier for faithful CPU semantics is
  `ogar-loco` (`OGAR/crates/ogar-loco`): a vocabulary-agnostic
  `Call = (function : value)` ABI, fixed-arity packed calls,
  nesting-by-reference, already used for Blockly/Scratch opcode streams.
  `ogar-vocab::ConceptDomain::BinaryLifting` at `0xC4XX` is reserved-empty
  and explicitly names Ghidra + r2sleigh as anticipated dual consumers.

## MISSING BUT MECHANICAL

1. 6502 arch dispatch in `r2sleigh-cli` — one match arm + a compiled `.sla`.
2. `sleigh-config` doesn't bundle 6502 — bundling convenience only.
3. A C64 PRG loader — trivial 2-byte header + bounds-checked copy. **Shipped
   in this repo as `c64-core::Memory::load_prg`.**

## ACTUAL ARCHITECTURAL GAP

Not a missing ABI primitive — a missing **transcoding seam** between R2IL
output and an OGAR carrier. Two options, in order of fit:

- (a) **Preferred, no new ABI surface**: treat each of the 65,536 addresses
  as a row in a `lance-graph-java` `RowStore` (byte-value lane +
  instruction-start-bit lane + opcode-class lane), fed by
  `r2sleigh_lift::Disassembler::lift_block()` output. Reuses
  `GraphHopTest`'s enforcement, the allocation gate, `lgj_hop`/`lgj_plan_eval`
  verbatim.
- (b) New substrate work (OGAR `ogar-loco` Call/FunctionBody vocabulary
  entries, `0xC4XX` classid minting) — real, scoped, pre-documented, but
  crosses the "ask before minting new classid concepts" line. **Gated on
  operator sign-off, not autonomous work.**

This repo builds (a) — Probes A and B — autonomously. Probe C's OGAR/classid
half is written up here as a spec, not implemented, until the operator says
go.

## Probe A — lift parity (partially DONE: `crates/c64-lift`)

**What's shipped:**

1. The vendored, unmodified `6502.slaspec`/`.pspec`/`.cspec`/`.ldefs` from
   `AdaWorldAPI/ghidra` live at `vendor/ghidra-6502/` (Apache-2.0, provenance
   in `vendor/ghidra-6502/NOTICE.md`).
2. `crates/c64-lift/build.rs` compiles `6502.slaspec` at build time via the
   `sleigh-compiler` crate (a Rust binding over Ghidra's own C++ SLEIGH
   compiler) — **no Gradle/Java Ghidra build needed**, contrary to the
   original plan's step 2. This was verified to actually work in-sandbox,
   not assumed: `SleighCompiler::compile` on the real `6502.slaspec`
   succeeds with only benign warnings (1 NOP constructor, one unreferenced
   `ADDR8` table).
3. `Disassembler::from_sla(sla_bytes, pspec_xml, "6502")` — exactly as
   planned, zero r2sleigh-lift code changes. `r2sleigh-lift` and `r2il` are
   pulled as git dependencies on `AdaWorldAPI/r2sleigh`'s **`master`**
   branch (not `main` — verified against the actual repo, the plan's
   assumption was wrong).
4. `c64-lift::lift()` wraps `lift_block()` over the fixture → `R2ILBlock`.
5. **Parity check (partial, not the full oracle below)**: the fixture's
   hand-computed BNE target ($0809) and JSR target ($0810) — see
   `c64-core::fixtures` — are asserted against the REAL lifted R2IL's
   `CBranch`/`Call` op targets. Both agree. Verified as a real falsifier by
   a disable-run (temporarily corrupting the expected constant, confirming
   the test fails, then restoring it) — not just asserted.

**What's NOT done** — the full oracle this section originally specified:

- No Ghidra headless-analyzer comparison. This checks r2sleigh's lift
  against hand-derived 6502 addressing-mode semantics, not against
  Ghidra's own native decode of the same bytes. That's a materially
  weaker parity claim: it proves r2sleigh + the vendored spec agree with
  *our* understanding of 6502 addressing, not that they agree with
  Ghidra's *own* independent decode path (which is the whole point of a
  parity oracle — two independent implementations of the same spec should
  never both be wrong the same way, but they CAN both encode a shared
  misunderstanding of the spec they're compiled from).
- No per-instruction address/size/mnemonic table comparison — only the two
  branch/call targets and a return-op count are checked.
- Running Ghidra headless (needs its full Gradle/Java build, or a
  pre-built Ghidra distribution) was not attempted this session — real,
  likely substantial effort (Ghidra's build is large), left for a future
  session rather than rushed.

## Probe B — physics (this repo, `crates/c64-core`)

- `Memory`: flat `[u8; 65536]`, `load_prg`. Shipped.
- `AddressMask`: packed bitset, `and`/`or`/`and_not`/`count`, one named
  `materialize_addresses` exit. Shipped.
- Next: populate two `AddressMask`s from a `lift_block()` run — "byte
  belongs to an instruction" and "byte is an instruction start" — as the
  connective tissue to Probe A, still without any Java/FFM involvement.

## Probe C — the transcoding seam (spec only, not implemented)

```
resident C64 bytes
        │
   6502 SLEIGH → P-code → R2IL          (Probe A)
        │
   ogar-loco Call/FunctionBody           ← operator-gated: new classid mint
   (R2ILOp → Call vocabulary entries,      under 0xC4XX BinaryLifting
    classid under 0xC4XX BinaryLifting)
        │
   ClassView (THINK arm)                 ← raw/decoded/banked memory readings
   reads the SAME resident bytes
        │
   c64-core AddressMask / RowStore        (Probe B)
        │
   Java/Panama semantic facade            (lance-graph-java, unmodified ABI)
```

**Success condition** (falsifier-shaped, not aspirational): one resident
address range must be simultaneously readable as (a) raw bytes via
`ClassView`, (b) an `ogar-loco` `Call` sequence carrying R2IL semantics, and
(c) a Java-facing view — zero second-source-of-truth divergence between
(a)/(b)/(c), zero per-instruction Java object population.

## Kill conditions

1. r2sleigh's generic P-code→R2IL match cannot faithfully round-trip a
   6502-specific construct (e.g. JSR's stack-relative `*:2 (SP-1) =
   inst_next` pattern) without a 6502-specific carve-out.
2. The RowStore/lane model can't express "byte value" and "decoded
   instruction" as two lanes without a lane-semantics collision.
3. Exposing `search(JSR)` genuinely requires per-instruction crossings
   because 6502 instruction boundaries are variable-length and only
   resolvable by walking, not by a bulk fixed-width predicate.
4. The public Java facade needs one object per decoded instruction to
   preserve required semantics (e.g. per-instruction branch-target metadata
   can't pack into fixed-width lanes).

None of these are triggered yet — Probe A hasn't run.

## Later, not gated: a2ui-rs as the eventual viewer

Once Probe C's `ClassView` projection exists, `AdaWorldAPI/a2ui-rs` is the
natural render target for a visual memory/disassembly view — it already
projects `ClassView`-resolved server state to a client via `NodeDelta`/
`ActionInvoke` with zero-serialization framing (`a2ui-server`/`a2ui-wasm`/
`a2ui-paint`), and its `Skin::Tile` skin already reads a geographic
`(x, y)` pair straight out of a 12-byte V3 facet register — structurally
close to "read a byte's page:offset coordinate out of the same register."
No work starts here until Probe C's ClassView projection exists; this is a
forward pointer, not a task in flight. Operator raised this
(2026-08-25 session) — don't let it drop between now and Probe C.

## GPL/licensing note

zinc64 (GPLv3), CrabSID, resid-rs are legitimate *behavioral reference*
sources for hardware register semantics (VIC-II/SID/CIA) — read, never
transcribed. Only Ghidra's Apache-2.0 SLEIGH specs are a lawful source for
anything that becomes a minted `ogar-vocab` concept.
