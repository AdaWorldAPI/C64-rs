//! Flat 64 KiB C64 resident address space — Probe B's physical substrate.
//!
//! Deliberately stupid on purpose (see `.claude/plans/c64-falsifier-v1.md`):
//! no bank switching, no VIC-II/SID/CIA I/O mapping, no cartridges. Just
//! `[u8; 65536]` with a bounds-checked PRG loader. Complexity is added only
//! when a later probe's fixture actually needs it.

use std::fmt;

/// Number of addressable bytes in the 6502's 16-bit address space.
pub const ADDRESS_SPACE_SIZE: usize = 1 << 16;

/// Error returned when a PRG's payload would run past the end of the
/// address space, or the PRG is too short to carry a load address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The PRG has fewer than 2 bytes, so no load address could be read.
    TooShortForHeader,
    /// `load_addr + payload.len()` overflows past `ADDRESS_SPACE_SIZE`.
    PayloadOverflows { load_addr: u16, payload_len: usize },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::TooShortForHeader => {
                write!(f, "PRG too short to contain a 2-byte load address header")
            }
            LoadError::PayloadOverflows {
                load_addr,
                payload_len,
            } => write!(
                f,
                "PRG payload of {payload_len} bytes at load address ${load_addr:04X} \
                 would run past the end of the 64 KiB address space"
            ),
        }
    }
}

impl std::error::Error for LoadError {}

/// The C64's flat 64 KiB resident byte array.
///
/// This is the "physics" layer only: it knows nothing about instruction
/// boundaries, opcodes, or hardware register semantics. Those are separate,
/// derived readings (`ClassView`-shaped, per the falsifier's Probe C design)
/// layered on top of this resident state, never folded into it.
pub struct Memory {
    bytes: Box<[u8; ADDRESS_SPACE_SIZE]>,
}

impl Memory {
    /// A freshly zeroed 64 KiB address space.
    pub fn new() -> Self {
        Self {
            bytes: Box::new([0u8; ADDRESS_SPACE_SIZE]),
        }
    }

    /// Read a single resident byte.
    pub fn read(&self, addr: u16) -> u8 {
        self.bytes[addr as usize]
    }

    /// Write a single resident byte.
    pub fn write(&mut self, addr: u16, value: u8) {
        self.bytes[addr as usize] = value;
    }

    /// Borrow the full resident state, zero-copy.
    pub fn as_slice(&self) -> &[u8; ADDRESS_SPACE_SIZE] {
        &self.bytes
    }

    /// Load a `.PRG`-shaped byte stream: the first two bytes (little-endian)
    /// give the load address, the remainder is copied verbatim starting
    /// there. Returns the load address and the length of the copied payload
    /// on success.
    ///
    /// This is deliberately the entire loader — no BASIC/KERNAL relocation,
    /// no bank awareness. See the module doc.
    pub fn load_prg(&mut self, prg: &[u8]) -> Result<(u16, usize), LoadError> {
        if prg.len() < 2 {
            return Err(LoadError::TooShortForHeader);
        }
        let load_addr = u16::from_le_bytes([prg[0], prg[1]]);
        let payload = &prg[2..];
        let end = load_addr as usize + payload.len();
        if end > ADDRESS_SPACE_SIZE {
            return Err(LoadError::PayloadOverflows {
                load_addr,
                payload_len: payload.len(),
            });
        }
        self.bytes[load_addr as usize..end].copy_from_slice(payload);
        Ok((load_addr, payload.len()))
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_memory_is_zeroed() {
        let mem = Memory::new();
        assert_eq!(mem.read(0x0000), 0);
        assert_eq!(mem.read(0xFFFF), 0);
    }

    #[test]
    fn read_write_round_trips_at_every_boundary() {
        let mut mem = Memory::new();
        for addr in [0x0000u16, 0x0001, 0x7FFF, 0x8000, 0xFFFE, 0xFFFF] {
            mem.write(addr, 0xAB);
            assert_eq!(mem.read(addr), 0xAB, "addr ${addr:04X} did not round-trip");
        }
    }

    #[test]
    fn load_prg_places_payload_at_its_declared_load_address() {
        let mut mem = Memory::new();
        // load addr $0801 (BASIC start), payload = [$A9, $10] (LDA #$10)
        let prg = [0x01, 0x08, 0xA9, 0x10];
        let (addr, len) = mem.load_prg(&prg).expect("valid PRG must load");
        assert_eq!(addr, 0x0801);
        assert_eq!(len, 2);
        assert_eq!(mem.read(0x0801), 0xA9);
        assert_eq!(mem.read(0x0802), 0x10);
        // nothing else was touched
        assert_eq!(mem.read(0x0800), 0);
        assert_eq!(mem.read(0x0803), 0);
    }

    #[test]
    fn load_prg_at_the_very_top_of_the_address_space_is_exact_not_off_by_one() {
        let mut mem = Memory::new();
        // load addr $FFFE, exactly 2 payload bytes fits flush against $FFFF.
        let prg = [0xFE, 0xFF, 0x11, 0x22];
        let (addr, len) = mem.load_prg(&prg).expect("flush-fit PRG must load");
        assert_eq!(addr, 0xFFFE);
        assert_eq!(len, 2);
        assert_eq!(mem.read(0xFFFE), 0x11);
        assert_eq!(mem.read(0xFFFF), 0x22);
    }

    #[test]
    fn load_prg_one_byte_past_the_top_is_rejected_not_wrapped() {
        let mut mem = Memory::new();
        // load addr $FFFE, 3 payload bytes — one byte would wrap past $FFFF.
        let prg = [0xFE, 0xFF, 0x11, 0x22, 0x33];
        let err = mem
            .load_prg(&prg)
            .expect_err("overflowing PRG must be rejected");
        assert_eq!(
            err,
            LoadError::PayloadOverflows {
                load_addr: 0xFFFE,
                payload_len: 3,
            }
        );
        // and memory must be untouched by the rejected load
        assert_eq!(mem.read(0xFFFE), 0);
    }

    #[test]
    fn a_prg_with_no_payload_loads_zero_bytes_without_error() {
        let mut mem = Memory::new();
        let prg = [0x00, 0x08]; // just a header, no payload
        let (addr, len) = mem.load_prg(&prg).expect("header-only PRG is valid, empty");
        assert_eq!(addr, 0x0800);
        assert_eq!(len, 0);
    }

    #[test]
    fn a_prg_shorter_than_the_header_is_rejected() {
        let mut mem = Memory::new();
        assert_eq!(
            mem.load_prg(&[0x01])
                .expect_err("1-byte PRG has no full header"),
            LoadError::TooShortForHeader
        );
        assert_eq!(
            mem.load_prg(&[]).expect_err("empty PRG has no header"),
            LoadError::TooShortForHeader
        );
    }
}
