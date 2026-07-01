// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 54
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_54.node.js

function testBuiltins_frag_54() {
    try {

        // finding all the letters of a text
        const story = "It's the Cheshire Cat: now I shall have somebody to talk to.";

        // Most explicit form
        story.match(/\p{General_Category=Letter}/gu);

        // It is not mandatory to use the property name for General categories
        story.match(/\p{Letter}/gu);

        // This is equivalent (short alias):
        story.match(/\p{L}/gu);

        // This is also equivalent (conjunction of all the subcategories using short aliases)
        story.match(/\p{Lu}|\p{Ll}|\p{Lt}|\p{Lm}|\p{Lo}/gu);
        } catch (e) {
        console.error(`[testBuiltins_frag_54] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_54();
}

module.exports = { testBuiltins_frag_54 };
