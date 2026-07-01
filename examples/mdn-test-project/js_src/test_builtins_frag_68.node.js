// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 68
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_68.node.js

function testBuiltins_frag_68() {
    try {

        (42).toString(2); // "101010" (binary)
        (13).toString(8); // "15" (octal)
        (0x42).toString(10); // "66" (decimal)
        (100000).toString(16); // "186a0" (hexadecimal)
        } catch (e) {
        console.error(`[testBuiltins_frag_68] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_68();
}

module.exports = { testBuiltins_frag_68 };
