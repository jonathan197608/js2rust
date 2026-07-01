// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 157
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_157.node.js

function testBuiltins_frag_157() {
    try {

        console.log(parseInt("123"));
        // 123 (default base-10)
        console.log(parseInt("123", 10));
        // 123 (explicitly specify base-10)
        console.log(parseInt("   123 "));
        // 123 (whitespace is ignored)
        console.log(parseInt("077"));
        // 77 (leading zeros are ignored)
        console.log(parseInt("1.9"));
        // 1 (decimal part is truncated)
        console.log(parseInt("ff", 16));
        // 255 (lower-case hexadecimal)
        console.log(parseInt("0xFF", 16));
        // 255 (upper-case hexadecimal with "0x" prefix)
        console.log(parseInt("xyz"));
        // NaN (input can't be converted to an integer)
        } catch (e) {
        console.error(`[testBuiltins_frag_157] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_157();
}

module.exports = { testBuiltins_frag_157 };
