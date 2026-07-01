// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 23
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_23.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_23() {

        // You cannot access properties on null or undefined

        const nullVar = null;
        nullVar.toString(); // TypeError: Cannot read properties of null
        String(nullVar); // "null"

        const undefinedVar = undefined;
        undefinedVar.toString(); // TypeError: Cannot read properties of undefined
        String(undefinedVar); // "undefined"
    }
