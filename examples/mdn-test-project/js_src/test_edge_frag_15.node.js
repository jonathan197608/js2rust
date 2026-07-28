// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 15

function testEdge_frag_15() {
    try {
    const obj = { a: 1, b: "two", c: null, d: undefined, e: [1, 2, { f: 3 }] };
    console.log(JSON.stringify(obj));
    console.log(JSON.stringify(obj, ["a", "e"]));
    console.log(JSON.stringify(obj, null, 2));
    console.log(JSON.stringify({ x: NaN, y: Infinity }));
    console.log(JSON.stringify([null, undefined, true, false]));
    } catch (e) {
        console.error(`[testEdge_frag_15] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_15();
}

module.exports = { testEdge_frag_15 };
