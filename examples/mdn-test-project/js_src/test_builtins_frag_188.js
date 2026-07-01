// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 188
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_188.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_188() {

        if (new Boolean(true)) {
          console.log("This log is printed.");
        }

        if (new Boolean(false)) {
          console.log("This log is ALSO printed.");
        }

        const myFalse = new Boolean(false); // myFalse is a Boolean object (not the primitive value false)
        const g = Boolean(myFalse); // g is true
        const myString = new String("Hello"); // myString is a String object
        const s = Boolean(myString); // s is true
    }
