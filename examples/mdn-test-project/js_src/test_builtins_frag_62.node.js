// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 62
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_62.node.js

function testBuiltins_frag_62() {
    try {

        const invalid = new Date("nothing");
        invalid.toISOString(); // RangeError: invalid date
        invalid.toJSON(); // RangeError: invalid date
        JSON.stringify({ date: invalid }); // RangeError: invalid date
        } catch (e) {
        console.error(`[testBuiltins_frag_62] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_62();
}

module.exports = { testBuiltins_frag_62 };
