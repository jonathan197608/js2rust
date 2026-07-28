// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 15
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_15() {
    const obj = { a: 1, b: "two", c: null, d: undefined, e: [1, 2, { f: 3 }] };
    console.log(JSON.stringify(obj));
    console.log(JSON.stringify(obj, ["a", "e"]));
    console.log(JSON.stringify(obj, null, 2));
    console.log(JSON.stringify({ x: NaN, y: Infinity }));
    console.log(JSON.stringify([null, undefined, true, false]));
}
