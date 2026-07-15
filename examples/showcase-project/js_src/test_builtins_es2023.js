// test_builtins_es2023.js
// Array ES2023 methods (toReversed, toSorted, toSpliced, with) have
// codegen issues: return ArrayList that can't be indexed, iterated,
// or have .length accessed in generated Zig. Fully tested via Rust
// unit tests. This file is a placeholder for future e2e coverage
// once the codegen issues are resolved.
