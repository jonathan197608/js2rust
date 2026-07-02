// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 62
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_62.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_62() {

                const date = 1;
const invalid = new Date("nothing");
        invalid.toISOString(); // RangeError: invalid date
        invalid.toJSON(); // RangeError: invalid date
        JSON.stringify({ date: invalid }); // RangeError: invalid date
    }
