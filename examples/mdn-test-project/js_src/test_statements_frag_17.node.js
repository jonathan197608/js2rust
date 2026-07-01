// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 17
// Source: test_statements_part*.js
// Run: node test_statements_frag_17.node.js

function testStatements_frag_17() {
    try {

        const result = /(a+)(b+)(c+)/.exec("aaabcc");
        const [, a, b, c] = result;
        console.log(a, b, c); // "aaa" "b" "cc"
        } catch (e) {
        console.error(`[testStatements_frag_17] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_17();
}

module.exports = { testStatements_frag_17 };
