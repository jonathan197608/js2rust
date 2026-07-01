// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 209
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_209.node.js

function testBuiltins_frag_209() {
    try {

        const d = new Date("1995-12-17T03:24:00");
        console.log(Number(d));
        } catch (e) {
        console.error(`[testBuiltins_frag_209] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_209();
}

module.exports = { testBuiltins_frag_209 };
