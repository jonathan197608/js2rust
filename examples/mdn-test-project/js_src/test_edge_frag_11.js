// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 11
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_11() {
    const sparse = [1, , 3, , 5];
    console.log(sparse.length);
    console.log(sparse[1]);
    console.log(sparse[3]);
    console.log(sparse.map(x => x * 2).join(","));
    console.log(sparse.filter(x => x !== undefined).join(","));
}
