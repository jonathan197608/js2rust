// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 205
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_205.node.js

function testBuiltins_frag_205() {
    try {

        Number("123"); // returns the number 123
        Number("123") === 123; // true

        Number("unicorn"); // NaN
        Number(undefined); // NaN
        } catch (e) {
        console.error(`[testBuiltins_frag_205] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_205();
}

module.exports = { testBuiltins_frag_205 };
