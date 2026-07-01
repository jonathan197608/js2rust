// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 16
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_16.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_16() {

        const object1 = {
          key: "value",
        };

        const object2 = {
          key: "value",
        };

        console.log(object1 === object2); // false
        console.log(object1 === object1); // true
    }
