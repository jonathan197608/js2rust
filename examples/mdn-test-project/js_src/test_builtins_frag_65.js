// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 65
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_65.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_65() {

        (77.1234).toExponential(-1); // RangeError
        (77.1234).toExponential(101); // RangeError

        (2.34).toFixed(-100); // RangeError
        (2.34).toFixed(1001); // RangeError

        (1234.5).toPrecision(-1); // RangeError
        (1234.5).toPrecision(101); // RangeError
    }
