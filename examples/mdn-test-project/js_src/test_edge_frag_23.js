// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 23
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_23() {
    const obj = {};
    for (let i = 0; i < 3; i++) {
        obj["key" + i] = i * 2;
    }
    obj["key1"] += 100;
    obj["key2"] **= 2;
    console.log(obj.key0);
    console.log(obj.key1);
    console.log(obj.key2);
    const arr = [1, 2, 3];
    arr[0] += 10;
    arr[1] <<= 1;
    arr[2] %= 2;
    console.log(arr.join(","));
}
