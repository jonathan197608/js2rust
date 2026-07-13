// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 219
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_219.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_219() {
    // BigInt truthiness
    if (0n) {
        console.log("truthy");
    } else {
        console.log("falsy");
    }
    if (12n) {
        console.log("truthy");
    } else {
        console.log("falsy");
    }
    console.log(!12n);
    console.log(!0n);
}
