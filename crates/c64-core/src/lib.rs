//! `c64-core` — the physics layer of the C64/6502 architecture falsifier.
//!
//! This crate deliberately does nothing clever: a flat 64 KiB resident
//! byte array (`memory`), a PRG loader, and a packed address bitmask
//! (`mask`) in the mask-native shape `lance-graph-java` already
//! establishes for row-native graph state. No SLEIGH, no P-code, no R2IL,
//! no OGAR `ClassView` — those are later, separate probes/crates. See
//! `.claude/plans/c64-falsifier-v1.md` for the full A/B/C probe sequence
//! and why this crate stops here on purpose.

pub mod fixtures;
pub mod mask;
pub mod memory;
pub mod row_store;

pub use mask::AddressMask;
pub use memory::{LoadError, Memory, ADDRESS_SPACE_SIZE};
pub use row_store::{Lane, RowStore};
