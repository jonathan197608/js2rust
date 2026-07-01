// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 35
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_35.node.js

function testBuiltins_frag_35() {
    try {

        function getLineTerminators(str) {
          return str.match(/[\r\n\u2028\u2029\q{\r\n}]/gv);
        }

        getLineTerminators(`
        A poem\r
        Is split\r\n
        Into many
        Stanzas
        `); // [ '\r', '\r\n', '\n' ]
        } catch (e) {
        console.error(`[testBuiltins_frag_35] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_35();
}

module.exports = { testBuiltins_frag_35 };
