// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 45
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_45.node.js

function testBuiltins_frag_45() {
    try {

        const variables = ["foo", "foo:bar", "  foo  "];

        function toAssignment(key) {
          if (isValidIdentifier(key)) {
            return `globalThis.${key} = undefined;`;
          }
          // JSON.stringify() escapes quotes and other special characters
          return `globalThis[${JSON.stringify(key)}] = undefined;`;
        }

        const statements = variables.map(toAssignment).join("\n");

        console.log(statements);
        // globalThis.foo = undefined;
        // globalThis["foo:bar"] = undefined;
        // globalThis["  foo  "] = undefined;
        } catch (e) {
        console.error(`[testBuiltins_frag_45] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_45();
}

module.exports = { testBuiltins_frag_45 };
