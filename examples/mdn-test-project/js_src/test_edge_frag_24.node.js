// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 24

function testEdge_frag_24() {
    try {
    const base = [1, 2, 3];
    const extended = [0, ...base, 4, 5];
    console.log(extended.join(","));
    function sum(...nums) {
        return nums.reduce((a, b) => a + b, 0);
    }
    console.log(sum(...base));
    console.log(sum(1, ...base, 4));
    const obj1 = { a: 1, b: 2 };
    const obj2 = { ...obj1, c: 3 };
    console.log(JSON.stringify(obj2));
    console.log(JSON.stringify({ ...obj1, a: 99 }));
    } catch (e) {
        console.error(`[testEdge_frag_24] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_24();
}

module.exports = { testEdge_frag_24 };
