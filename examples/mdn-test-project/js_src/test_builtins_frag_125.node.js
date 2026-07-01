// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 125
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_125.node.js

function testBuiltins_frag_125() {
    try {

        obj.foo.bar; // "baz"
        // or alternatively
        obj["foo"]["bar"]; // "baz"

        // computed properties require square brackets
        obj.foo["bar" + i]; // "baz2"
        // or as template literal
        obj.foo[`bar${i}`]; // "baz2"
        } catch (e) {
        console.error(`[testBuiltins_frag_125] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_125();
}

module.exports = { testBuiltins_frag_125 };
