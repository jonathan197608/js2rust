// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 11

function testEdge_frag_11() {
    try {
    const sparse = [1, , 3, , 5];
    console.log(sparse.length);
    console.log(sparse[1]);
    console.log(sparse[3]);
    console.log(sparse.map(x => x * 2).join(","));
    console.log(sparse.filter(x => x !== undefined).join(","));
    } catch (e) {
        console.error(`[testEdge_frag_11] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_11();
}

module.exports = { testEdge_frag_11 };
