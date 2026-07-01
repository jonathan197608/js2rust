// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 120
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_120.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_120() {

        const t = Temporal.Now.instant();
        "" + t; // Throws TypeError
        `${t}`; // '2022-07-31T04:48:56.113918308Z'
        "".concat(t); // '2022-07-31T04:48:56.113918308Z'
    }
