// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 10
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_10.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_10() {

        "1" != 1; // false
        1 != "1"; // false
        0 != false; // false
        0 != null; // true
        0 != undefined; // true
        0 != !!null; // false, look at Logical NOT operator
        0 != !!undefined; // false, look at Logical NOT operator
        null != undefined; // false

        const number1 = new Number(3);
        const number2 = new Number(3);
        number1 != 3; // false
        number1 != number2; // true
    }
