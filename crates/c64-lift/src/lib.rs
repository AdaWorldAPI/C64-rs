//! Probe A — lifts 6502 machine code to R2IL via the real, unmodified
//! Ghidra SLEIGH spec vendored at `vendor/ghidra-6502/`, compiled at build
//! time (`build.rs`) through `sleigh-compiler`, and loaded through
//! `r2sleigh-lift`'s already-generic `Disassembler::from_sla`.
//!
//! This crate writes ZERO 6502 decode logic. Every semantic fact about
//! what an opcode does comes from the vendored SLEIGH spec; every P-code→
//! R2IL translation rule comes from r2sleigh's existing arch-agnostic
//! opcode table. See `CLAUDE.md`'s hard rules.

use r2sleigh_lift::disasm::Disassembler;

const SLA_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/6502.sla"));
const PSPEC: &str = include_str!("../../../vendor/ghidra-6502/data/languages/6502.pspec");

/// Build a `Disassembler` for the vendored 6502 spec. Cheap to call once
/// per use; not cached here — a caller doing many lifts should hold onto
/// the returned value rather than re-parsing the `.sla` per call.
pub fn disassembler() -> r2sleigh_lift::Result<Disassembler> {
    Disassembler::from_sla(SLA_BYTES, PSPEC, "6502")
}

/// Lift `code` (starting at `addr`) to an R2IL block, using the real
/// SLEIGH-compiled 6502 spec.
pub fn lift(code: &[u8], addr: u64) -> r2sleigh_lift::Result<r2il::R2ILBlock> {
    let disasm = disassembler()?;
    disasm.lift_block(code, addr, code.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use c64_core::fixtures;
    use r2il::opcode::R2ILOp;
    use r2il::space::SpaceId;

    #[test]
    fn the_vendored_spec_loads_without_error() {
        disassembler().expect("the real Ghidra 6502 SLEIGH spec must load");
    }

    #[test]
    fn lifting_the_fixture_produces_a_nonempty_r2il_block() {
        let block = lift(fixtures::CODE_BYTES, fixtures::LOAD_ADDR as u64)
            .expect("lift_block must succeed on the fixture");
        assert!(
            !block.ops.is_empty(),
            "a 10-instruction fixture must lift to a nonempty R2IL block"
        );
    }

    /// This is Probe A's parity check, in miniature: the fixture's BNE
    /// target was hand-computed (see fixtures.rs) as $0809 (LOOP_ADDR).
    /// If r2sleigh's P-code→R2IL translation of the 6502's relative-branch
    /// addressing mode disagreed with that hand computation, this would
    /// fail — proving the two independent derivations (hand-assembly
    /// semantics vs. real Ghidra SLEIGH + r2sleigh lift) actually agree.
    #[test]
    fn the_lifted_conditional_branch_targets_the_hand_computed_loop_address() {
        let block = lift(fixtures::CODE_BYTES, fixtures::LOAD_ADDR as u64)
            .expect("lift_block must succeed on the fixture");

        let cbranch_targets: Vec<u64> = block
            .ops
            .iter()
            .filter_map(|op| match op {
                R2ILOp::CBranch { target, .. } => Some(target.offset),
                _ => None,
            })
            .collect();

        assert_eq!(
            cbranch_targets,
            vec![fixtures::LOOP_ADDR as u64],
            "exactly one CBranch op must target the hand-computed loop address"
        );
    }

    /// Same shape, for JSR: the fixture's JSR operand was hand-computed as
    /// $0810 (SUB_ADDR). r2sleigh's `Call` op target must agree.
    #[test]
    fn the_lifted_call_targets_the_hand_computed_sub_address() {
        let block = lift(fixtures::CODE_BYTES, fixtures::LOAD_ADDR as u64)
            .expect("lift_block must succeed on the fixture");

        let call_targets: Vec<u64> = block
            .ops
            .iter()
            .filter_map(|op| match op {
                R2ILOp::Call { target } => Some(target.offset),
                _ => None,
            })
            .collect();

        assert_eq!(
            call_targets,
            vec![fixtures::SUB_ADDR as u64],
            "exactly one Call op must target the hand-computed sub address"
        );
    }

    /// Anti-vacuity for the two tests above: the fixture has exactly two
    /// RTS instructions (main's and sub's), so the lift must produce
    /// exactly two Return ops — proving the block wasn't silently
    /// truncated or partially lifted.
    #[test]
    fn the_lifted_block_has_exactly_two_return_ops_matching_the_two_rts_instructions() {
        let block = lift(fixtures::CODE_BYTES, fixtures::LOAD_ADDR as u64)
            .expect("lift_block must succeed on the fixture");

        let return_count = block
            .ops
            .iter()
            .filter(|op| matches!(op, R2ILOp::Return { .. }))
            .count();

        assert_eq!(
            return_count, 2,
            "the fixture has exactly two RTS instructions (main's and sub's)"
        );
    }

    /// Data-movement parity: `STA $0400` must produce exactly one op
    /// writing into the RAM address space at offset $0400. This is a
    /// second, independent kind of semantic claim from the branch/call
    /// target checks above — it exercises the SLEIGH spec's addressing-
    /// mode-to-RAM-space mapping, not its control-flow constructors.
    #[test]
    fn the_lifted_block_writes_into_ram_at_the_stas_hand_computed_address() {
        let block = lift(fixtures::CODE_BYTES, fixtures::LOAD_ADDR as u64)
            .expect("lift_block must succeed on the fixture");

        let ram_writes: Vec<u64> = block
            .ops
            .iter()
            .filter_map(|op| match op {
                R2ILOp::Copy { dst, .. } if dst.space == SpaceId::Ram => Some(dst.offset),
                R2ILOp::Store { addr, .. } if addr.space == SpaceId::Ram => Some(addr.offset),
                _ => None,
            })
            .collect();

        assert_eq!(
            ram_writes,
            vec![0x0400],
            "STA $0400 is the fixture's only RAM-space write; its target \
             must be exactly $0400, the address hand-computed from the \
             instruction's own operand bytes (00 04, little-endian)"
        );
    }

    /// `AND #$0F` must lift through an `IntAnd` op — proving the SLEIGH
    /// spec's logical-AND semantics actually reach R2IL, not just that
    /// SOME ops were produced (the anti-vacuity half of the data-movement
    /// coverage this test file otherwise lacked).
    #[test]
    fn the_lifted_block_contains_an_int_and_op_for_the_and_instruction() {
        let block = lift(fixtures::CODE_BYTES, fixtures::LOAD_ADDR as u64)
            .expect("lift_block must succeed on the fixture");

        let and_count = block
            .ops
            .iter()
            .filter(|op| matches!(op, R2ILOp::IntAnd { .. }))
            .count();

        assert_eq!(
            and_count, 1,
            "the fixture has exactly one AND instruction (AND #$0F)"
        );
    }
}
