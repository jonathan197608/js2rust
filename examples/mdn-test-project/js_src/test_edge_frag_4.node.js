// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 4

function testEdge_frag_4() {
    try {
    const map = new Map();
    for (let i = 0; i < 5; i++) {
        map.set("key" + i, i * 10);
    }
    map.set("key2", 999);
    map.delete("key0");
    console.log(map.size);
    for (const [k, v] of map) {
        console.log(k + "=" + v);
    }
    console.log(map.get("key1"));
    console.log(map.has("key0"));
    } catch (e) {
        console.error(`[testEdge_frag_4] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_4();
}

module.exports = { testEdge_frag_4 };
