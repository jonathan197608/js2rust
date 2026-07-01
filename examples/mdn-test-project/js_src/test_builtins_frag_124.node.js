// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 124
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_124.node.js

function testBuiltins_frag_124() {
    try {

        square(2); // 4

        greet("Howdy"); // "Howdy"

        log({ obj: "value" }); // { obj: "value" }
        } catch (e) {
        console.error(`[testBuiltins_frag_124] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_124();
}

module.exports = { testBuiltins_frag_124 };
