// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 60
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_60.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_60() {

        function A() {
          console.log("called A");
          return false;
        }
        function B() {
          console.log("called B");
          return true;
        }

        console.log(B() || A());
        // Logs "called B" due to the function call,
        // then logs true (which is the resulting value of the operator)
    }
