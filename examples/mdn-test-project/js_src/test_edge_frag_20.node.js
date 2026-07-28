// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 20

function testEdge_frag_20() {
    try {
    const { a = 1, b = 2, c = 3 } = { a: 10, c: 30 };
    console.log(a, b, c);
    const [first, second = "def", ...rest] = ["one", , "three", "four"];
    console.log(first);
    console.log(second);
    console.log(rest.join(","));
    const { x: { y } } = { x: { y: 42 } };
    console.log(y);
    } catch (e) {
        console.error(`[testEdge_frag_20] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_20();
}

module.exports = { testEdge_frag_20 };
