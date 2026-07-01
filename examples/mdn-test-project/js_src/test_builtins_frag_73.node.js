// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 73
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_73.node.js

function testBuiltins_frag_73() {
    try {

        "use strict";

        const args = [1, 2, 3];
        console.log(Math.max(...args));

        function foo(...args) {
          console.log(args);
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_73] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_73();
}

module.exports = { testBuiltins_frag_73 };
