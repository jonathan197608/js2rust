// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 22

function testEdge_frag_22() {
    try {
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
    } catch (e) {
        console.error(`[testEdge_frag_22] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_22();
}

module.exports = { testEdge_frag_22 };
