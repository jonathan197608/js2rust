// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 77
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_77.node.js

function testBuiltins_frag_77() {
    try {

        const iterable = [10, 20, 30];

        for (let value of iterable) {
          value += 50;
          console.log(value);
        }
        // 60
        // 70
        // 80
        } catch (e) {
        console.error(`[testBuiltins_frag_77] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_77();
}

module.exports = { testBuiltins_frag_77 };
