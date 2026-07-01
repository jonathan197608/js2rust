// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 28
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_28.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_28() {

        const arr = [1, 2, 3];

        // Assign all array values to 0
        for (let i = 0; i < arr.length; arr[i++] = 0) /* empty statement */ ;

        console.log(arr);
        // [0, 0, 0]
    }
