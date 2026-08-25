//! A packed bitmask over the 64 KiB address space.
//!
//! This is the local (non-FFM) shape of the mask-native currency this
//! project inherits from `lance-graph-java`'s ABI doctrine: "which
//! addresses conduct" is a bitset, never a `Vec<u16>` of selected
//! addresses. A full 65,536-bit mask is exactly 8 KiB — see
//! `.claude/plans/c64-falsifier-v1.md` Probe B.
//!
//! `materialize_addresses` is the one deliberately named, O(n) exit from
//! this currency — mirroring `lance-graph-java`'s `materializeRows()`
//! naming convention exactly, so the same reflective-allowlist enforcement
//! pattern (a public surface may only leave mask-native form through a
//! method whose name starts with `materialize`) can be ported here
//! verbatim in a later probe.

use crate::memory::ADDRESS_SPACE_SIZE;

const WORDS: usize = ADDRESS_SPACE_SIZE / 64;

/// A bitset over every address in the 64 KiB address space.
#[derive(Clone, PartialEq, Eq)]
pub struct AddressMask {
    words: Box<[u64; WORDS]>,
}

impl AddressMask {
    /// An empty mask — no address selected.
    pub fn empty() -> Self {
        Self {
            words: Box::new([0u64; WORDS]),
        }
    }

    /// A full mask — every address selected.
    pub fn full() -> Self {
        Self {
            words: Box::new([u64::MAX; WORDS]),
        }
    }

    /// Whether `addr` is selected.
    pub fn contains(&self, addr: u16) -> bool {
        let addr = addr as usize;
        (self.words[addr / 64] >> (addr % 64)) & 1 == 1
    }

    /// Select `addr`.
    pub fn set(&mut self, addr: u16) {
        let addr = addr as usize;
        self.words[addr / 64] |= 1u64 << (addr % 64);
    }

    /// Deselect `addr`.
    pub fn clear(&mut self, addr: u16) {
        let addr = addr as usize;
        self.words[addr / 64] &= !(1u64 << (addr % 64));
    }

    /// Number of selected addresses. Bulk (word-popcount), not per-address.
    pub fn count(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }

    /// Word-wise AND — bulk, ∝ WORDS not ∝ selected-address-count.
    pub fn and(&self, other: &Self) -> Self {
        let mut out = Self::empty();
        for i in 0..WORDS {
            out.words[i] = self.words[i] & other.words[i];
        }
        out
    }

    /// Word-wise OR — bulk.
    pub fn or(&self, other: &Self) -> Self {
        let mut out = Self::empty();
        for i in 0..WORDS {
            out.words[i] = self.words[i] | other.words[i];
        }
        out
    }

    /// Word-wise AND-NOT (`self` minus `other`) — bulk.
    pub fn and_not(&self, other: &Self) -> Self {
        let mut out = Self::empty();
        for i in 0..WORDS {
            out.words[i] = self.words[i] & !other.words[i];
        }
        out
    }

    /// The deliberate, explicitly-named, O(n) exit from mask-native form.
    /// Every other method on this type stays mask-native; this is the one
    /// place a caller may pay for a `Vec<u16>` of selected addresses.
    pub fn materialize_addresses(&self) -> Vec<u16> {
        let mut out = Vec::with_capacity(self.count() as usize);
        for (word_idx, word) in self.words.iter().enumerate() {
            let mut bits = *word;
            while bits != 0 {
                let bit = bits.trailing_zeros();
                out.push((word_idx * 64 + bit as usize) as u16);
                bits &= bits - 1; // clear lowest set bit
            }
        }
        out
    }
}

impl Default for AddressMask {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_mask_selects_nothing() {
        let m = AddressMask::empty();
        assert_eq!(m.count(), 0);
        assert!(!m.contains(0x0000));
        assert!(!m.contains(0xFFFF));
    }

    #[test]
    fn full_mask_selects_every_address() {
        let m = AddressMask::full();
        assert_eq!(m.count(), ADDRESS_SPACE_SIZE as u32);
        assert!(m.contains(0x0000));
        assert!(m.contains(0xFFFF));
        assert!(m.contains(0x8000));
    }

    #[test]
    fn set_and_clear_round_trip_at_word_boundaries() {
        let mut m = AddressMask::empty();
        for addr in [0x0000u16, 0x003F, 0x0040, 0x8000, 0xFFFF] {
            m.set(addr);
            assert!(m.contains(addr), "${addr:04X} should be set");
            m.clear(addr);
            assert!(!m.contains(addr), "${addr:04X} should be cleared");
        }
    }

    #[test]
    fn and_or_and_not_agree_with_naive_per_address_definitions() {
        let mut a = AddressMask::empty();
        let mut b = AddressMask::empty();
        for addr in [0x10u16, 0x20, 0x30] {
            a.set(addr);
        }
        for addr in [0x20u16, 0x30, 0x40] {
            b.set(addr);
        }

        let and = a.and(&b);
        let or = a.or(&b);
        let and_not = a.and_not(&b);

        for addr in 0u16..=0x50 {
            let expected_and = a.contains(addr) && b.contains(addr);
            let expected_or = a.contains(addr) || b.contains(addr);
            let expected_and_not = a.contains(addr) && !b.contains(addr);
            assert_eq!(and.contains(addr), expected_and, "AND at ${addr:04X}");
            assert_eq!(or.contains(addr), expected_or, "OR at ${addr:04X}");
            assert_eq!(
                and_not.contains(addr),
                expected_and_not,
                "AND-NOT at ${addr:04X}"
            );
        }
    }

    #[test]
    fn materialize_addresses_returns_exactly_the_selected_set_in_ascending_order() {
        let mut m = AddressMask::empty();
        let selected = [0x0005u16, 0x0040, 0x00FF, 0x8000, 0xFFFF];
        for &addr in &selected {
            m.set(addr);
        }
        assert_eq!(m.materialize_addresses(), selected.to_vec());
    }

    #[test]
    fn count_matches_materialized_length_on_a_sparse_mask() {
        let mut m = AddressMask::empty();
        for addr in (0u16..ADDRESS_SPACE_SIZE as u16).step_by(97) {
            m.set(addr);
        }
        assert_eq!(m.count() as usize, m.materialize_addresses().len());
    }
}
