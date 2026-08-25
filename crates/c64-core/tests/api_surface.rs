//! Structural enforcement of the mask-native currency rule, ported from
//! `lance-graph-java`'s `GraphHopTest` reflective allowlist (Rust has no
//! runtime reflection, so this is a source-text scan instead — deliberately
//! scoped to `src/mask.rs` only, since that's the module the mask-native
//! doctrine actually governs; `fixtures.rs`'s `prg_bytes()` returning
//! `Vec<u8>` is a small, fixed-size test constant builder, not a
//! materialization of population-scale state, and is out of scope on
//! purpose).
//!
//! The rule: on `AddressMask`, a public function may return `Vec<u16>`
//! (a materialized set of addresses) ONLY if its name starts with
//! `materialize`. Every other public function must stay mask-native —
//! returning `Self`, `bool`, `u32`, or `()`, never a `Vec` of the
//! population.
//!
//! This is a best-effort line scanner, not a real Rust parser — it is
//! scoped to `crates/c64-core/src/mask.rs`'s current single-line
//! function-signature style. If a future signature spans multiple lines,
//! this test's parsing (not its intent) needs to grow with it.

use std::fs;
use std::path::Path;

#[test]
fn mask_rs_never_returns_vec_from_a_method_not_named_materialize() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mask.rs");
    let source = fs::read_to_string(&path).expect("src/mask.rs must exist");

    let mut checked_any_pub_fn = false;
    let mut saw_a_real_materialize_case = false;

    for line in source.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("pub fn ") {
            continue;
        }
        checked_any_pub_fn = true;

        let after_fn = &trimmed["pub fn ".len()..];
        let name_end = after_fn.find(['(', '<']).unwrap_or(after_fn.len());
        let name = &after_fn[..name_end];

        let returns_vec = trimmed.contains("-> Vec<") || trimmed.contains("Vec <");

        if returns_vec {
            saw_a_real_materialize_case = true;
            assert!(
                name.starts_with("materialize"),
                "pub fn `{name}` in mask.rs returns a Vec (materializes the \
                 population) but is not named starting with `materialize` — \
                 this is exactly the mask-native currency violation \
                 lance-graph-java's GraphHopTest exists to catch"
            );
        } else {
            assert!(
                !name.starts_with("materialize"),
                "pub fn `{name}` is named `materialize*` but does not return \
                 a Vec — the naming convention exists specifically to flag \
                 the O(n) exit points, a misleading name defeats that"
            );
        }
    }

    // Anti-vacuity: the scan must have actually found public functions, and
    // must have actually exercised the positive case (a real materialize*
    // method that DOES return a Vec), or this test proves nothing.
    assert!(
        checked_any_pub_fn,
        "mask.rs must have public functions to scan"
    );
    assert!(
        saw_a_real_materialize_case,
        "mask.rs must contain at least one materialize*-named Vec-returning \
         method for this test to exercise its positive case"
    );
}
