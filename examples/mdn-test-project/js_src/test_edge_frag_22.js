// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 22
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_22() {
    let x = null;
    x ??= 42;
    console.log(x);
    let y = 0;
    y ||= 10;
    console.log(y);
    let z = 5;
    z &&= z + 1;
    console.log(z);
    let obj = {};
    obj.prop ??= "default";
    console.log(obj.prop);
    const arr = [];
    arr[0] ??= "first";
    arr[0] ??= "second";
    console.log(arr[0]);
}
