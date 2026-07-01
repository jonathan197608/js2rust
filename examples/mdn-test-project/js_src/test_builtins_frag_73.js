// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 73
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_73.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_73() {

        "use strict";

        const args = [1, 2, 3];
        console.log(Math.max(...args));

        function foo(...args) {
          console.log(args);
        }
    }
