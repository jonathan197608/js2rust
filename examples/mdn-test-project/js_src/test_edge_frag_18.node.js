// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 18

function testEdge_frag_18() {
    try {
    const callback = (x) => x * 2;
    const nullCb = null;
    console.log(callback?.(5));
    console.log(nullCb?.(5));
    console.log(callback?.(5) ?? 0);
    const arr = [1, 2, 3];
    console.log(arr?.map(x => x + 1)?.join(","));
    console.log(arr?.find?.(x => x === 2));
    } catch (e) {
        console.error(`[testEdge_frag_18] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_18();
}

module.exports = { testEdge_frag_18 };
