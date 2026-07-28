// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 28
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_28() {
    const map = new Map([["a", 1], ["b", 2], ["c", 3]]);
    console.log([...map.keys()].join(","));
    console.log([...map.values()].join(","));
    console.log([...map.entries()].map(e => e[0] + ":" + e[1]).join(","));
    const set1 = new Set([1, 2, 2, 3, 3, 3]);
    console.log(set1.size);
    console.log([...set1].join(","));
    const set2 = new Set([3, 4, 5]);
    const union = new Set([...set1, ...set2]);
    console.log([...union].sort().join(","));
    console.log([...map].reduce((sum, [k, v]) => sum + v, 0));
}
