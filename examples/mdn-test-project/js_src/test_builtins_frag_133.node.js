// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 133
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_133.node.js

function testBuiltins_frag_133() {
    try {

        // Only setting the prototype once
        const obj = { __proto__: { a: 1 } };

        // These syntaxes all create a property called "__proto__" and can coexist
        // They would overwrite each other and the last one is actually used
        const __proto__ = null;
        const obj2 = {
          ["__proto__"]: {},
          __proto__,
          __proto__() {},
          get __proto__() {
            return 1;
          },
        };
        } catch (e) {
        console.error(`[testBuiltins_frag_133] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_133();
}

module.exports = { testBuiltins_frag_133 };
