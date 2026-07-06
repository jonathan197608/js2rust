// zigir/mod.rs
// ZigIR — structured intermediate representation between AST and Zig source code.
//
// Pipeline: AST → Lowerer (AST→ZigIR) → [Opt Passes] → Emitter (ZigIR→String) → Zig source
//
// IR type system is defined here. Lowering is in `lower/`, emitting in `emit/`,
// optimization/validation passes in `passes/`.

pub mod builtins;
pub mod emit;
pub mod ident;
pub mod kinds;
pub mod lower;
pub mod ops;
pub mod passes;
pub mod source_span;
pub mod types;
