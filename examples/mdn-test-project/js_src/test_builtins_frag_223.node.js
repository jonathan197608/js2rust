// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 223
// Source: ICU localeCompare
// Run: node test_builtins_frag_223.node.js

function testBuiltins_frag_223() {
    try {

        // localeCompare: basic ordering
        const r1 = "a".localeCompare("b"); // -1
        const r2 = "b".localeCompare("a"); // 1
        const r3 = "a".localeCompare("a"); // 0

        console.log(r1, r2, r3);
        } catch (e) {
        console.error(`[testBuiltins_frag_223] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_223();
}

module.exports = { testBuiltins_frag_223 };
