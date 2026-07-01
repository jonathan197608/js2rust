// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 147
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_147.node.js

function testBuiltins_frag_147() {
    try {

        [..."abc".matchAll(/./g)]; // [[ "a" ], [ "b" ], [ "c" ]]
        "abc".replaceAll(/./g, "f"); // "fff"

        const existingPattern = /./;
        const newPattern = new RegExp(
          existingPattern.source,
          `${existingPattern.flags}g`,
        );
        "abc".replaceAll(newPattern, "f"); // "fff"
        } catch (e) {
        console.error(`[testBuiltins_frag_147] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_147();
}

module.exports = { testBuiltins_frag_147 };
