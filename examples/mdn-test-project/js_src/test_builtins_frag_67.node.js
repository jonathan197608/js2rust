// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 67
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_67.node.js

function testBuiltins_frag_67() {
    try {

        (42).toString(0);
        (42).toString(1);
        (42).toString(37);
        (42).toString(150);
        // You cannot use a string like this for formatting:
        (12071989).toString("MM-dd-yyyy");
        } catch (e) {
        console.error(`[testBuiltins_frag_67] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_67();
}

module.exports = { testBuiltins_frag_67 };
