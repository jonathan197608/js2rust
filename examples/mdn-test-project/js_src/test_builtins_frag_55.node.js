// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 55
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_55.node.js

function testBuiltins_frag_55() {
    try {

        // Ù¢ is the digit 2 in Arabic-Indic notation
        // while it is predominantly written within the Arabic script
        // it can also be written in the Thaana script

        "Ù¢".match(/\p{Script=Thaana}/u);
        // null as Thaana is not the predominant script

        "Ù¢".match(/\p{Script_Extensions=Thaana}/u);
        // ["Ù¢", index: 0, input: "Ù¢", groups: undefined]
        } catch (e) {
        console.error(`[testBuiltins_frag_55] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_55();
}

module.exports = { testBuiltins_frag_55 };
