// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 5

function testEdge_frag_5() {
    try {
    const inner = new Set([1, 2, 3]);
    const outer = new Map();
    outer.set("set", inner);
    outer.set("num", 42);
    const retrieved = outer.get("set");
    retrieved.add(4);
    retrieved.delete(1);
    console.log(retrieved.size);
    console.log(retrieved.has(2));
    console.log(retrieved.has(5));
    console.log(outer.get("num"));
    } catch (e) {
        console.error(`[testEdge_frag_5] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_5();
}

module.exports = { testEdge_frag_5 };
