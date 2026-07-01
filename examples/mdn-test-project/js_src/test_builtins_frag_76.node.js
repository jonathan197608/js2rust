// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 76
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_76.node.js

function testBuiltins_frag_76() {
    try {

        "use strict";
        class DocArchiver {}

        // SyntaxError: class is a reserved identifier
        // (throws in older browsers only, e.g. Firefox 44 and older)
        } catch (e) {
        console.error(`[testBuiltins_frag_76] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_76();
}

module.exports = { testBuiltins_frag_76 };
