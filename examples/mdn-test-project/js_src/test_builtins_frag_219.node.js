// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 219
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_219.node.js

function testBuiltins_frag_219() {
    try {

        if (0n) {
          console.log("Hello from the if!");
        } else {
          console.log("Hello from the else!");
        }
        // "Hello from the else!"

        0n || 12n; // 12n
        0n && 12n; // 0n
        Boolean(0n); // false
        Boolean(12n); // true
        !12n; // false
        !0n; // true
        } catch (e) {
        console.error(`[testBuiltins_frag_219] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_219();
}

module.exports = { testBuiltins_frag_219 };
