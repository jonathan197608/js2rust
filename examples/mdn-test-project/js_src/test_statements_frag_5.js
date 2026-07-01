// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 5
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_5.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_5() {

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
    }
