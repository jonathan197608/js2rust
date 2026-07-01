// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 13
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_13.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_13() {

        function random(min, max) {
          const num = Math.floor(Math.random() * (max - min + 1)) + min;
          return num;
        }

        random(1, 10);
    }
