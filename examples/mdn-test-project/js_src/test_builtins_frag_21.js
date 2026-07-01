// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 21
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_21.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_21() {

        const s1 = "2 + 2"; // creates a string primitive
        const s2 = new String("2 + 2"); // creates a String object
        console.log(eval(s1)); // returns the number 4
        console.log(eval(s2)); // returns the string "2 + 2"
    }
