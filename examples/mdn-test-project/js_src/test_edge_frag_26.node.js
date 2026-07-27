// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 26

function testEdge_frag_26() {
    try {

    const name = "World";
    const count = 3;
    const items = ["a", "b", "c"];
    console.log(`Hello ${name}!`);
    console.log(`Count: ${count}, Items: ${items.join("-")}`);
    console.log(`Math: ${1 + 2 * 3}`);
    console.log(`Nested: ${`inner ${1 + 1}`}`);
    console.log(`Multi
    line
    string`);

    } catch (e) {
        console.error(`[testEdge_frag_26] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_26();
}

module.exports = { testEdge_frag_26 };
