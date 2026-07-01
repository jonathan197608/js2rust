// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 35
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_35.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_35() {

        // main.js

        console.log(myValue); // 1
        console.log(myModule.myValue); // 1
        setTimeout(() => {
          console.log(myValue); // 2; my-module has updated its value
          console.log(myModule.myValue); // 2
          myValue = 3; // TypeError: Assignment to constant variable.
          // The importing module can only read the value but can't re-assign it.
        }, 1000);
    }
