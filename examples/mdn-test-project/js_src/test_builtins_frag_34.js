// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 34
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_34.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_34() {

        function isHexadecimal(str) {
          return /^[0-9A-F]+$/i.test(str);
        }

        isHexadecimal("2F3"); // true
        isHexadecimal("beef"); // true
        isHexadecimal("undefined"); // false
    }
