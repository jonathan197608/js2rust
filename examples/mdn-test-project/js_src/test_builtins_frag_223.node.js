// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 223
// Source: ICU localeCompare
// Run: node test_builtins_frag_223.node.js

function testBuiltins_frag_223() {
    try {

        // localeCompare: basic ordering
        console.log("a".localeCompare("b"));
        console.log("b".localeCompare("a"));
        console.log("a".localeCompare("a"));
        } catch (e) {
        console.error(`[testBuiltins_frag_223] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_223();
}

module.exports = { testBuiltins_frag_223 };
