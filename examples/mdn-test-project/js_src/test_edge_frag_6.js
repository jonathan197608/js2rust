// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 6
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_6() {
    const obj = {};
    obj["string_key"] = "value1";
    obj[123] = "value2";
    obj[true] = "value3";
    obj[null] = "value4";
    console.log(Object.keys(obj).length);
    console.log(obj["123"]);
    console.log(obj["true"]);
    console.log(obj["null"]);
    delete obj["string_key"];
    console.log(Object.keys(obj).length);
    console.log("string_key" in obj);
}
