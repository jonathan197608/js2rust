// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 5
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_5() {
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
}
