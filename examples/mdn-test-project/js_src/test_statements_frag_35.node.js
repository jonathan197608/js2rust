// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 35
// Source: test_statements_part*.js
// Run: node test_statements_frag_35.node.js

function testStatements_frag_35() {
    try {

        // main.js

        console.log(myValue); // 1
        console.log(myModule.myValue); // 1
        setTimeout(() => {
          console.log(myValue); // 2; my-module has updated its value
          console.log(myModule.myValue); // 2
          myValue = 3; // TypeError: Assignment to constant variable.
          // The importing module can only read the value but can't re-assign it.
        }, 1000);
        } catch (e) {
        console.error(`[testStatements_frag_35] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_35();
}

module.exports = { testStatements_frag_35 };
