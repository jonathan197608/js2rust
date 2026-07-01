// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 0
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_0.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_0() {

        const output = void 1;
        console.log(output);

        void console.log("expression evaluated");

        void (function iife() {
          console.log("iife is executed");
        })();

        void function test() {
          console.log("test function executed");
        };
        try {
          test();
        } catch (e) {
          console.log("test function is not defined");
        }
    }
