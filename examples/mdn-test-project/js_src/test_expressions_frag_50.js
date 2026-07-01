// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 50
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_50.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_50() {

        function A() {
          console.log("called A");
          return false;
        }
        function B() {
          console.log("called B");
          return true;
        }

        console.log(A() && B());
        // Logs "called A" to the console due to the call for function A,
        // && evaluates to false (function A returns false), then false is logged to the console;
        // the AND operator short-circuits here and ignores function B
    }
