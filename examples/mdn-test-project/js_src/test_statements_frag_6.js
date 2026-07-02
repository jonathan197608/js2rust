// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 6
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_6.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_6() {

                const innerBlock = 1;
        const outerBlock = 1;
outerBlock: {
          innerBlock: {
            console.log("1");
            break outerBlock; // breaks out of both innerBlock and outerBlock
            console.log(":-("); // skipped
          }
          console.log("2"); // skipped
        }
    }
