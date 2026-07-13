// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 227
// Source: Math.sign
// Run with Node.js: node test_builtins_frag_227.node.js
// Transpile with js2rust: cargo build -p mdn-test-project
// Note: -0 case omitted because i64 domain cannot represent -0.0

export function testBuiltins_frag_227() {
    console.log(Math.sign(3));
    console.log(Math.sign(-3));
    console.log(Math.sign(0));
    console.log(Math.sign(42));
    console.log(Math.sign(-42));
}
