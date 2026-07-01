// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 81
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_81.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_81() {

        typeof null; // "object" (not "null" for legacy reasons)
        typeof undefined; // "undefined"
        null === undefined; // false
        null == undefined; // true
        null === null; // true
        null == null; // true
        !null; // true
        Number.isNaN(1 + null); // false
        Number.isNaN(1 + undefined); // true
    }
