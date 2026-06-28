// app.js — Minimal entry point for js2rust
// This file just exports a dummy function.
// The actual tests are in test_expressions.js, test_statements.js, test_builtins.js
// and are called directly from Rust (src/main.rs).

export function dummy() {
    return 0;
}
