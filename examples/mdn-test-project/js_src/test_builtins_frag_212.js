// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 212
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_212.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_212() {
    // typeof bigint
    console.log(typeof 1n === "bigint");
    console.log(typeof BigInt("1") === "bigint");
}
