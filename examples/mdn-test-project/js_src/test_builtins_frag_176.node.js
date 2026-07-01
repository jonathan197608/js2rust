// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 176
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_176.node.js

function testBuiltins_frag_176() {
    try {

        const set1 = ";/?:@&=+$,#"; // Reserved Characters
        const set2 = "-.!~*'()"; // Unreserved Marks
        const set3 = "ABC abc 123"; // Alphanumeric Characters + Space

        console.log(encodeURI(set1)); // ;/?:@&=+$,#
        console.log(encodeURI(set2)); // -.!~*'()
        console.log(encodeURI(set3)); // ABC%20abc%20123 (the space gets encoded as %20)

        console.log(encodeURIComponent(set1)); // %3B%2C%2F%3F%3A%40%26%3D%2B%24%23
        console.log(encodeURIComponent(set2)); // -.!~*'()
        console.log(encodeURIComponent(set3)); // ABC%20abc%20123 (the space gets encoded as %20)
        } catch (e) {
        console.error(`[testBuiltins_frag_176] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_176();
}

module.exports = { testBuiltins_frag_176 };
