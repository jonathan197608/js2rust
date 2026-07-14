// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 219
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_219.node.js

function testBuiltins_frag_219() {
    try {
        if (0n) {
            console.log("truthy");
        } else {
            console.log("falsy");
        }
        if (12n) {
            console.log("truthy");
        } else {
            console.log("falsy");
        }
        console.log(!12n);
        console.log(!0n);
    } catch (e) {
        console.error(`[testBuiltins_frag_219] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_219();
}

module.exports = { testBuiltins_frag_219 };
