// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 123
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_123.node.js

function testBuiltins_frag_123() {
    try {

        function square(number) {
          return number * number;
        }

        function greet(greeting) {
          return greeting;
        }

        function log(arg) {
          console.log(arg);
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_123] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_123();
}

module.exports = { testBuiltins_frag_123 };
