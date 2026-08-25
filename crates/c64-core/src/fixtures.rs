//! The hand-assembled 6502 fixture from the falsifier mission brief, used
//! by Probe A's parity oracle (not yet wired) and available now as a Probe B
//! smoke test that `Memory::load_prg` places every byte where the assembly
//! actually puts it.
//!
//! ```text
//!         LDA #$10        ; A9 10
//!         AND #$0F        ; 29 0F
//!         STA $0400       ; 8D 00 04
//!         LDX #$03        ; A2 03
//! loop:   DEX             ; CA
//!         BNE loop        ; D0 FD  (target loop, from $080C: offset -3)
//!         JSR sub         ; 20 10 08
//!         RTS             ; 60
//! sub:    INX             ; E8
//!         RTS             ; 60
//! ```
//!
//! Every offset below was hand-computed against 6502 addressing rules
//! (relative branches are signed offsets from the address *after* the
//! 2-byte branch instruction; JSR takes an absolute little-endian operand)
//! and is asserted, not assumed — see the tests.

/// Load address for [`PRG_BYTES`]: `$0800`, unencumbered by a BASIC stub.
pub const LOAD_ADDR: u16 = 0x0800;

/// The fixture's machine code, in order, starting at [`LOAD_ADDR`].
pub const CODE_BYTES: &[u8] = &[
    0xA9, 0x10, // $0800 LDA #$10
    0x29, 0x0F, // $0802 AND #$0F
    0x8D, 0x00, 0x04, // $0804 STA $0400
    0xA2, 0x03, // $0807 LDX #$03
    0xCA, // $0809 loop: DEX
    0xD0, 0xFD, // $080A BNE loop  (target $0809)
    0x20, 0x10, 0x08, // $080C JSR $0810
    0x60, // $080F RTS
    0xE8, // $0810 sub: INX
    0x60, // $0811 RTS
];

/// The address of the `loop:` label (`DEX`).
pub const LOOP_ADDR: u16 = 0x0809;

/// The address of the `sub:` label (`INX`).
pub const SUB_ADDR: u16 = 0x0810;

/// A PRG-shaped byte stream: [`LOAD_ADDR`] as a little-endian 2-byte header,
/// then [`CODE_BYTES`] — ready for [`crate::Memory::load_prg`].
pub fn prg_bytes() -> Vec<u8> {
    let mut prg = Vec::with_capacity(2 + CODE_BYTES.len());
    prg.extend_from_slice(&LOAD_ADDR.to_le_bytes());
    prg.extend_from_slice(CODE_BYTES);
    prg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Memory;

    #[test]
    fn the_fixture_loads_at_its_declared_address_with_every_byte_intact() {
        let mut mem = Memory::new();
        let (addr, len) = mem
            .load_prg(&prg_bytes())
            .expect("the fixture is a well-formed PRG");
        assert_eq!(addr, LOAD_ADDR);
        assert_eq!(len, CODE_BYTES.len());
        for (i, &expected) in CODE_BYTES.iter().enumerate() {
            let a = LOAD_ADDR + i as u16;
            assert_eq!(mem.read(a), expected, "byte at ${a:04X} mismatched");
        }
    }

    #[test]
    fn the_bne_relative_offset_actually_targets_the_loop_label() {
        // BNE's operand is a signed 8-bit offset from the address *after*
        // the 2-byte branch instruction. The branch instruction (D0 FD)
        // sits at $080A, so "after" is $080C; 0xFD as i8 is -3.
        let branch_addr: u16 = 0x080A;
        let after_branch = branch_addr + 2;
        let offset = CODE_BYTES[(branch_addr - LOAD_ADDR) as usize + 1] as i8;
        let target = (after_branch as i32 + offset as i32) as u16;
        assert_eq!(
            target, LOOP_ADDR,
            "BNE's computed target must be the loop label"
        );
    }

    #[test]
    fn the_jsr_operand_actually_targets_the_sub_label() {
        let jsr_addr: u16 = 0x080C;
        let opcode_offset = (jsr_addr - LOAD_ADDR) as usize;
        assert_eq!(CODE_BYTES[opcode_offset], 0x20, "must be JSR's opcode");
        let lo = CODE_BYTES[opcode_offset + 1];
        let hi = CODE_BYTES[opcode_offset + 2];
        let target = u16::from_le_bytes([lo, hi]);
        assert_eq!(
            target, SUB_ADDR,
            "JSR's operand must be the sub label's address"
        );
    }

    #[test]
    fn the_fixture_is_exactly_18_bytes_of_code_with_no_gaps_or_overlaps() {
        // sub's final RTS is the last byte; this pins the fixture's total
        // extent so a future edit that shifts an addressing mode (and thus
        // instruction length) is forced to update every hand-computed
        // offset above rather than silently drifting.
        assert_eq!(CODE_BYTES.len(), 18);
        assert_eq!(LOAD_ADDR + CODE_BYTES.len() as u16 - 1, 0x0811);
    }
}
