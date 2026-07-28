// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 16

function testEdge_frag_16() {
    try {
    const parsed = JSON.parse('{"a":1,"b":[true,null,"str"],"c":{"d":3.14}}');
    console.log(parsed.a);
    console.log(parsed.b.join(","));
    console.log(parsed.c.d);
    const arr = JSON.parse('[1, 2.5, null, true, "text", {"x": 1}]');
    console.log(arr.length);
    console.log(arr[5].x);
    console.log(arr[2]);
    } catch (e) {
        console.error(`[testEdge_frag_16] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_16();
}

module.exports = { testEdge_frag_16 };
