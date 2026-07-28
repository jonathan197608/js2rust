// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 17
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_17() {
    const obj = { a: { b: { c: 42 } }, d: null, e: [1, 2, 3] };
    console.log(obj?.a?.b?.c);
    console.log(obj?.a?.b?.c?.d);
    console.log(obj?.d?.x);
    console.log(obj?.e?.[1]);
    console.log(obj?.f?.g);
    console.log(obj?.a?.b?.c ?? 0);
    console.log(obj?.d ?? "def");
}
