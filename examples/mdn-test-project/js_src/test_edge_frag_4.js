// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 4
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_4() {
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
}
