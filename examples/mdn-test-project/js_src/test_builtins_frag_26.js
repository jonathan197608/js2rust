// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 26
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_26.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_26() {

        const buffer = new ArrayBuffer(16);
        const view = new DataView(buffer, 0);

        view.setInt16(1, 42);
        view.getInt16(1); // 42
    }
