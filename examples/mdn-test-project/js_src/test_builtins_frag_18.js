// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 18
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_18.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_18() {

        const a = "a";
        const b = "b";
        if (a < b) {
          // true
          console.log(`${a} is less than ${b}`);
        } else if (a > b) {
          console.log(`${a} is greater than ${b}`);
        } else {
          console.log(`${a} and ${b} are equal.`);
        }
    }
