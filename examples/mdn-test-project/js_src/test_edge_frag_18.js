// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 18
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_18() {
    const callback = (x) => x * 2;
    const nullCb = null;
    console.log(callback?.(5));
    console.log(nullCb?.(5));
    console.log(callback?.(5) ?? 0);
    const arr = [1, 2, 3];
    console.log(arr?.map(x => x + 1)?.join(","));
    console.log(arr?.find?.(x => x === 2));
}
