// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 16
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_16() {
    const parsed = JSON.parse('{"a":1,"b":[true,null,"str"],"c":{"d":3.14}}');
    console.log(parsed.a);
    console.log(parsed.b.join(","));
    console.log(parsed.c.d);
    const arr = JSON.parse('[1, 2.5, null, true, "text", {"x": 1}]');
    console.log(arr.length);
    console.log(arr[5].x);
    console.log(arr[2]);
}
