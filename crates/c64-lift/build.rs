//! Compiles the vendored `6502.slaspec` into a `.sla` at build time via
//! `sleigh-compiler` — the Rust binding over Ghidra's own C++ SLEIGH
//! compiler. No 6502 decode logic is written here; this only invokes the
//! upstream compiler on the upstream spec (see `vendor/ghidra-6502/NOTICE.md`).

use sleigh_compiler::{SleighCompiler, SleighCompilerOptions};
use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/c64-lift has two ancestors up to the workspace root");
    let input = workspace_root.join("vendor/ghidra-6502/data/languages/6502.slaspec");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("set by cargo"));
    let output = out_dir.join("6502.sla");

    println!("cargo:rerun-if-changed={}", input.display());

    let mut compiler = SleighCompiler::new(SleighCompilerOptions::default());
    let response = compiler
        .compile(&input, &output)
        .unwrap_or_else(|e| panic!("failed to compile {}: {e}", input.display()));

    for warning in &response.warnings {
        println!("cargo:warning=6502.slaspec: {warning}");
    }
}
