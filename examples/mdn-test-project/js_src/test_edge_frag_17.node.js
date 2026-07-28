// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 17

function testEdge_frag_17() {
    try {
    const obj = { a: { b: { c: 42 } }, d: null, e: [1, 2, 3] };
    console.log(obj?.a?.b?.c);
    console.log(obj?.a?.b?.c?.d);
    console.log(obj?.d?.x);
    console.log(obj?.e?.[1]);
    console.log(obj?.f?.g);
    console.log(obj?.a?.b?.c ?? 0);
    console.log(obj?.d ?? "def");
    } catch (e) {
        console.error(`[testEdge_frag_17] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_17();
}

module.exports = { testEdge_frag_17 };
