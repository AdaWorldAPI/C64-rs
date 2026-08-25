//! `RowStore` — the two-lane join of resident bytes and address-level
//! structure, prefigured for Probe C's eventual `lance-graph-java`
//! `RowStore` handoff.
//!
//! This is deliberately NOT the real `lance-graph-java` ABI type (no FFM,
//! no native crossing) — it's the local, in-process shape the falsifier's
//! Probe B/C bridge needs before that integration exists: proof that a
//! byte-value lane (`Memory`) and a derived structural lane
//! (`AddressMask`, e.g. "instruction starts") can be joined and queried
//! together without either lane copying the other, and without the join
//! ever materializing a per-address population.

use crate::mask::AddressMask;
use crate::memory::Memory;

/// A named, independently-addressable structural lane over the resident
/// address space (e.g. "instruction starts", "reachable", "code vs data").
///
/// Mirrors `lance-graph-java`'s lane concept in miniature: a lane is a
/// mask, never a per-address collection.
pub struct Lane {
    name: &'static str,
    mask: AddressMask,
}

impl Lane {
    pub fn new(name: &'static str, mask: AddressMask) -> Self {
        Self { name, mask }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn mask(&self) -> &AddressMask {
        &self.mask
    }
}

/// Joins one [`Memory`] (the byte-value lane) with zero or more named
/// [`Lane`]s (derived structural masks) over the *same* resident address
/// space.
///
/// The join is by reference conceptually: every lane is a mask over the
/// same 65,536 addresses `memory` holds, never a copy of the bytes. Adding
/// a lane costs one `AddressMask` (8 KiB, fixed, population-size
/// independent of how many addresses are actually selected) — never a
/// `Vec` sized to the selection.
pub struct RowStore {
    memory: Memory,
    lanes: Vec<Lane>,
}

impl RowStore {
    /// Take ownership of a [`Memory`] with no structural lanes yet.
    pub fn new(memory: Memory) -> Self {
        Self {
            memory,
            lanes: Vec::new(),
        }
    }

    /// Borrow the byte-value lane.
    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    /// Attach a named structural lane. Replaces any existing lane of the
    /// same name.
    pub fn add_lane(&mut self, name: &'static str, mask: AddressMask) {
        self.lanes.retain(|l| l.name != name);
        self.lanes.push(Lane::new(name, mask));
    }

    /// Look up a lane by name.
    pub fn lane(&self, name: &str) -> Option<&Lane> {
        self.lanes.iter().find(|l| l.name == name)
    }

    /// Number of attached lanes.
    pub fn lane_count(&self) -> usize {
        self.lanes.len()
    }

    /// Read a resident byte AND check whether `addr` is selected in the
    /// named lane, in one call — the shape a bulk "read value at every
    /// selected address" query would use, proven here at single-address
    /// granularity as the seam a future bulk crossing composes from.
    pub fn read_if_in_lane(&self, lane_name: &str, addr: u16) -> Option<u8> {
        let lane = self.lane(lane_name)?;
        if lane.mask().contains(addr) {
            Some(self.memory.read(addr))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    fn fixture_row_store() -> RowStore {
        let mut memory = Memory::new();
        memory
            .load_prg(&fixtures::prg_bytes())
            .expect("fixture PRG must load");
        let mut store = RowStore::new(memory);
        store.add_lane("instruction_start", fixtures::instruction_start_mask());
        store
    }

    #[test]
    fn a_fresh_row_store_has_no_lanes() {
        let store = RowStore::new(Memory::new());
        assert_eq!(store.lane_count(), 0);
        assert!(store.lane("instruction_start").is_none());
    }

    #[test]
    fn adding_a_lane_makes_it_findable_by_name() {
        let store = fixture_row_store();
        assert_eq!(store.lane_count(), 1);
        let lane = store.lane("instruction_start").expect("lane must exist");
        assert_eq!(lane.name(), "instruction_start");
        assert_eq!(
            lane.mask().count() as usize,
            fixtures::INSTRUCTION_START_ADDRS.len()
        );
    }

    #[test]
    fn adding_a_lane_with_the_same_name_replaces_not_duplicates() {
        let mut store = fixture_row_store();
        assert_eq!(store.lane_count(), 1);
        store.add_lane("instruction_start", AddressMask::empty());
        assert_eq!(store.lane_count(), 1, "must replace, not accumulate");
        assert_eq!(
            store
                .lane("instruction_start")
                .expect("lane must still exist")
                .mask()
                .count(),
            0,
            "the replacement mask must actually take effect"
        );
    }

    #[test]
    fn read_if_in_lane_returns_the_byte_only_when_the_address_is_selected() {
        let store = fixture_row_store();
        // $0800 is an instruction start (LDA opcode byte) -> Some.
        assert_eq!(
            store.read_if_in_lane("instruction_start", 0x0800),
            Some(0xA9)
        );
        // $0801 is LDA's operand byte, NOT an instruction start -> None.
        assert_eq!(store.read_if_in_lane("instruction_start", 0x0801), None);
        // The byte is still readable directly through memory(), proving
        // the lane gates the QUERY, not the underlying byte's existence.
        assert_eq!(store.memory().read(0x0801), 0x10);
    }

    #[test]
    fn read_if_in_lane_on_an_unknown_lane_name_is_none_not_a_panic() {
        let store = fixture_row_store();
        assert_eq!(store.read_if_in_lane("no_such_lane", 0x0800), None);
    }
}
