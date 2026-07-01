// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 66
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_66.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_66() {

        (77.1234).toExponential(4); // 7.7123e+1
        (77.1234).toExponential(2); // 7.71e+1

        (2.34).toFixed(1); // 2.3
        (2.35).toFixed(1); // 2.4 (note that it rounds up in this case)

        (5.123456).toPrecision(5); // 5.1235
        (5.123456).toPrecision(2); // 5.1
        (5.123456).toPrecision(1); // 5
    }
