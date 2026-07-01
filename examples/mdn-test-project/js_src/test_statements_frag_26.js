// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 26
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_26.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_26() {

        const array = [1, 2, 3];

        // Assign all array values to 0
        for (let i = 0; i < array.length; array[i++] = 0 /* empty statement */);

        console.log(array);
    }
