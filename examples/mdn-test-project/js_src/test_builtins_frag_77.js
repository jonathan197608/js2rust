// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 77
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_77.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_77() {

        const iterable = [10, 20, 30];

        for (let value of iterable) {
          value += 50;
          console.log(value);
        }
        // 60
        // 70
        // 80
    }
