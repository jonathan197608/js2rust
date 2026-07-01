// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 5
// Source: test_statements_part*.js
// Run: node test_statements_frag_5.node.js

function testStatements_frag_5() {
    try {

        const food = "sushi";

        switch (food) {
          case "sushi":
            console.log("Sushi is originally from Japan.");
            break;
          case "pizza":
            console.log("Pizza is originally from Italy.");
            break;
          default:
            console.log("I have never heard of that dish.");
            break;
        }
        } catch (e) {
        console.error(`[testStatements_frag_5] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_5();
}

module.exports = { testStatements_frag_5 };
