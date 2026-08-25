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

**⊘ SUPERSESSION FOUND UPSTREAM (2026-08-25) — the lift half of this was
already ruled retired, and this repo took the sanctioned path by luck.**

`AdaWorldAPI/lance-graph-java` carries a dated ruling,
`E-LGJ-GHIDRA-G1-G2-SUPERSEDED-BY-R2IL-1` (2026-08-18, recorded in its
`.claude/board/LATEST_STATE.md`): its own planned Ghidra waves — a
bespoke `analyzeHeadless` lift script plus a hand-rolled LE program-image
format — are **superseded, not merely deprioritised**, because *"direct
r2il/r2ssa consumption solves the upstream seam."*

Read against this repo: **`crates/c64-lift` is the sanctioned shape**
(r2sleigh consuming Ghidra's own unmodified `.sla`), and the
`analyzeHeadless` route this session spent effort trying to build is the
retired one. That was arrived at by accident, not by reading the ruling —
recorded here so the next session does not re-derive it or, worse,
re-attempt the retired path.

**The nuance that survives, and it matters:**
`lance-graph-java/.claude/plans/ghidra-integration-v1.md` names **two**
distinct Ghidra roles, and only the first is superseded:

1. **Ghidra as lift-time compiler** (its G1/G2) — SUPERSEDED by direct
   r2sleigh/R2IL consumption. This is the half c64-lift already replaces.
2. **Ghidra as parity oracle** (its G4) — **NOT superseded.** The
   instrument is Ghidra's own in-tree sequential `PcodeEmulator`
   (`Ghidra/Framework/Emulation/src/main/java/ghidra/pcode/emu/`), which
   executes the same P-code the lift produces. That plan describes it as
   the tesseract-rs byte-parity method transplanted: diff against the
   reference *implementation*, not only against our own scalar rewrite.

So Probe A's still-missing oracle is **`PcodeEmulator`, not
`analyzeHeadless`** — a better instrument than the one this session was
reaching for, and still gated on a real Ghidra build (below).

**A standing boundary in that same plan, recorded for awareness:**
*"this session does not build toward r2sleigh, ruff, or R2IL"* — its
r2sleigh/R2IL integration is arriving via a separate `ruff_r2il` arm
(`AdaWorldAPI/ruff` PR #94 landed; PR2/PR3 pending as of 2026-08-18).
That boundary is scoped to lance-graph-java, so this repo is not in
violation — but two arms are now building r2sleigh integration
independently and a future session should reconcile them rather than run
both designs in parallel.

**Ghidra-headless feasibility, checked read-only (not attempted) —
2026-08-25:** confirmed genuinely blocked, not just "not yet tried."

- `Ghidra/RuntimeScripts/support/analyzeHeadless` (the headless CLI
  entrypoint) exists as source and delegates to `launch.sh`, which
  resolves its Java classpath from either an assembled distribution's
  `lib/` directory (built by Gradle's `assembleDistribution` task) or a
  `gradle prepDev` dev-classpath generation step. **Neither exists in this
  checkout** — no `.jar` files anywhere, no `support/` directory outside
  `gradle/support` (build tooling, not the runtime one), no git tags
  suggesting a downloaded release.
- `pyghidra` (checked on PyPI, versions 3.1.0 down to 0.0.0 all listed)
  does **not** bundle Ghidra — it's a bridge library that requires
  `GHIDRA_INSTALL_DIR` pointing at an already-built distribution. It would
  not shortcut the same build requirement.
- **Verdict: genuinely blocked pending a future session with real build
  budget**, not a same-session oversight. Ghidra's own build (Gradle,
  Java, likely tens of minutes to hours depending on what's cached) is a
  different scale of effort than `sleigh-compiler`'s standalone C++
  compile, which is what made Probe A's SLEIGH-spec compilation tractable
  in a single session. Do not conflate the two when scoping future work —
  compiling ONE `.slaspec` and running Ghidra's FULL headless analyzer
  pipeline are unrelated amounts of effort, even though both ultimately
  touch "Ghidra."

## Probe B — physics (this repo, `crates/c64-core`)

- `Memory`: flat `[u8; 65536]`, `load_prg`. Shipped.
- `AddressMask`: packed bitset, `and`/`or`/`and_not`/`count`, one named
  `materialize_addresses` exit. Shipped.
- `RowStore`: joins `Memory` with named `AddressMask` lanes
  (`crates/c64-core/src/row_store.rs`), wired against the fixture's
  `instruction_start` lane. Shipped.
- A source-text materialization-allowlist enforcement test
  (`crates/c64-core/tests/api_surface.rs`), the Rust equivalent of
  `lance-graph-java`'s `GraphHopTest` reflective allowlist. Shipped.
- Next: extend the `RowStore` lane population to draw from a REAL
  `c64-lift::lift()` run (opcode-class / instruction-length lanes derived
  from the actual R2IL block), not just the fixture's hand-transcribed
  `instruction_start_mask()` — this is the actual Probe A/B/C connective
  tissue the plan always intended, now unblocked since Probe A is real.

## Session summary — 2026-08-24/25 autonomous overnight session

8 PRs merged, all test-covered, all clippy/fmt-clean, every parity claim
verified via at least one disable-run (red-then-green), not just asserted:

1. Scaffold: `c64-core` physics (`Memory`, `AddressMask`), doctrine, this
   plan doc.
2. Docs: noted `a2ui-rs` as the eventual Probe C viewer target.
3. Bridged the fixture's known instruction boundaries into `AddressMask`.
4. Ported `lance-graph-java`'s materialization-allowlist enforcement to a
   Rust source-text scan.
5. Added `RowStore`, joining `Memory` + named `AddressMask` lanes.
6. **Probe A actually shipped**: vendored the real, unmodified Ghidra 6502
   SLEIGH spec (Apache-2.0), compiled it at build time via `sleigh-compiler`
   (no Ghidra Gradle build needed — a real finding, corrects the plan's
   original assumption), lifted it through `r2sleigh-lift`'s existing
   generic `Disassembler::from_sla`, and checked the fixture's
   hand-computed BNE/JSR targets against the real lifted R2IL.
7. Extended Probe A's parity coverage to data movement (`STA`, `AND`).
8. Extended Probe A's parity coverage to a second register (`LDX`/`DEX`),
   catching and fixing a wrong assumption about the lift's `Unique`-space
   temporary shape along the way — a small, real instance of "verify
   before assuming" catching itself.

**What's shipped and real**: a genuine, running, falsifiable 6502→R2IL
lift, using zero hand-written decode logic, with 7 independent parity
claims (2 control-flow, 2 data-movement, 2 register, 1 nonempty-block
sanity) each proven to actually discriminate correct from incorrect
behavior via disable-run.

**What's still open, honestly**:
- The full Ghidra-headless parity oracle (see above) — genuinely blocked
  on build effort, not scoped away.
- Probe C (the OGAR/`ogar-loco`/classid transcoding seam) — spec only,
  correctly gated on operator sign-off for classid minting (see
  `CLAUDE.md`'s hard rules). Nothing here should proceed without that.
- `RowStore`'s lane population is still fixture-hand-transcribed, not yet
  drawing from a real `lift()` run — the natural next increment once a
  future session picks this back up, now that Probe A exists to draw from.
- No end-to-end demonstration of Probe C's stated success condition (one
  resident address range readable as raw bytes / `ogar-loco` Call sequence
  / Java-facing view with zero divergence) — this remains entirely
  unbuilt, correctly, pending the operator-gated classid decision.

Every hard rule in `CLAUDE.md` held throughout: no 6502 decoder was
written (SLEIGH + r2sleigh's existing generic table did all decode work),
no classid was minted, no sibling repo was modified, no GPL code was
transcribed (only Apache-2.0 Ghidra SLEIGH specs were vendored, with
provenance in `vendor/ghidra-6502/NOTICE.md`).

## Probe C — the transcoding seam (spec only, not implemented)

```
resident C64 bytes
        │
   6502 SLEIGH → P-code → R2IL          (Probe A)
        │
   ruff_r2il intake arm                  ← the EXISTING ore→furnace→slag
   (FunctionBehavior::from_blocks over      pipeline (AdaWorldAPI/ruff,
    the same R2ILBlocks → FlatFact rows      crates/ruff_r2il) — do not
    + addressed slag → sink → SoA V4)        build a second seam
        │
   ogar-loco Call/FunctionBody           ← classid homes now SPLIT:
   (R2ILOp → Call vocabulary entries)      0xC4XX BinaryLifting = lift/IR
                                           0xC6XX Mmio = platform hardware
                                           (minted 2026-08-25, OGAR #284)
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

## GPL/licensing note (fence widened 2026-08-25, operator-approved)

zinc64 (GPLv3), Frodo (GPLv2), CrabSID, resid-rs are legitimate *behavioral
reference* sources for hardware register semantics (VIC-II/SID/CIA) — read,
never transcribed. Lawful sources for anything that becomes a minted
`ogar-vocab` concept: Ghidra's Apache-2.0 SLEIGH specs, permissively-licensed
implementations (`AdaWorldAPI/rust64`, MIT), and vendor datasheets. The
original SLEIGH-only wording assumed the only hardware references were GPL
emulators; rust64 being MIT widened the set, not the rule.

## Probe C classid homes (recorded 2026-08-25)

`ConceptDomain::Mmio` (`0xC6XX`) is MINTED (OGAR PR #284): `mmio_chip`
(0xC601), `mmio_register` (0xC602), `rom_image` (0xC603),
`machine_memory_map` (0xC604) — container kinds only, per the 0x08XX OCR
precedent. `0xC4XX BinaryLifting` remains the (still zero-row) lift/IR
home. The hard rule above survives the mint verbatim: concrete registers
and the 56 mnemonics are content rows / `ogar-loco` `FnIndex` entries,
never classids.

Probe C's intake route is `ruff_r2il` (sibling-checkout-only crate — its
path deps escape the ruff repo, so the harvest lives THERE, path-unified
with r2sleigh, never as a git dep here: two r2il SourceIds are two
incompatible type universes).
