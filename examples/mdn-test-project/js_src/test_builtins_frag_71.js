// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 71
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_71.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_71() {
    try {
        "abc".repeat(-1); // RangeError
    } catch (e) {
        // RangeError expected
    }
}
