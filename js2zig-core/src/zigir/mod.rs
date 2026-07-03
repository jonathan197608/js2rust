// zigir/mod.rs
// ZigIR — structured intermediate representation between AST and Zig source code.
//
// Pipeline: AST → Lowerer (AST→ZigIR) → [Opt Passes] → Emitter (ZigIR→String) → Zig source
//
// This module defines the IR type system only. Lowering and emitting are in
// separate sub-modules (zigir/lower/, zigir/emit/) added in later stages.

pub mod builtins;
pub mod ident;
pub mod kinds;
pub mod ops;
pub mod source_span;
pub mod types;
