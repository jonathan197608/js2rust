// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 0

function testEdge_frag_0() {
    try {
    console.log(true + 1);
    console.log("3" + 1);
    console.log("3" - 1);
    console.log(null + 1);
    console.log(undefined + 1);
    console.log(true + "2" + 3);
    console.log(1 + 2 + "3" + 4);
    } catch (e) {
        console.error(`[testEdge_frag_0] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_0();
}

module.exports = { testEdge_frag_0 };
