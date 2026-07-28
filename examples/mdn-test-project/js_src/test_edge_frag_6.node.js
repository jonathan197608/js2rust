// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 6

function testEdge_frag_6() {
    try {
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
    } catch (e) {
        console.error(`[testEdge_frag_6] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_6();
}

module.exports = { testEdge_frag_6 };
