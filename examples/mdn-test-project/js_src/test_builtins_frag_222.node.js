// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 222
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_222.node.js

function testBuiltins_frag_222() {
    try {

        const replacer = (key, value) =>
          typeof value === "bigint" ? { $bigint: value.toString() } : value;

        const data = {
          number: 1,
          big: 18014398509481982n,
        };
        const stringified = JSON.stringify(data, replacer);

        console.log(stringified);
        // {"number":1,"big":{"$bigint":"18014398509481982"}}
        } catch (e) {
        console.error(`[testBuiltins_frag_222] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_222();
}

module.exports = { testBuiltins_frag_222 };
