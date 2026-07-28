// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 7

function testEdge_frag_7() {
    try {
    const result = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        .filter(x => x % 2 === 0)
        .map(x => x * x)
        .filter(x => x > 10)
        .reduce((acc, x) => acc + x, 0);
    console.log(result);
    } catch (e) {
        console.error(`[testEdge_frag_7] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_7();
}

module.exports = { testEdge_frag_7 };
