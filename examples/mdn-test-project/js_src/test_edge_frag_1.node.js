// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 1

function testEdge_frag_1() {
    try {
    console.log(1.5 | 2.7);
    console.log("3" & 1);
    console.log(0.1 + 0.2 | 0);
    console.log(-0 | 0);
    console.log(~"5");
    console.log(2 ** 31 | 0);
    console.log(true << 1);
    console.log(3.99 >> 1);
    } catch (e) {
        console.error(`[testEdge_frag_1] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_1();
}

module.exports = { testEdge_frag_1 };
