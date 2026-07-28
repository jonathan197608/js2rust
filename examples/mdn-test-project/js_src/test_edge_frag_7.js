// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 7
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_7() {
    const result = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        .filter(x => x % 2 === 0)
        .map(x => x * x)
        .filter(x => x > 10)
        .reduce((acc, x) => acc + x, 0);
    console.log(result);
}
