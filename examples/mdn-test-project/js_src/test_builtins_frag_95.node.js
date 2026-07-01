// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 95
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_95.node.js

function testBuiltins_frag_95() {
    try {

        JSON.parse("[1, 2, 3, 4,]");
        JSON.parse('{"foo": 1,}');
        // SyntaxError JSON.parse: unexpected character
        // at line 1 column 14 of the JSON data
        } catch (e) {
        console.error(`[testBuiltins_frag_95] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_95();
}

module.exports = { testBuiltins_frag_95 };
