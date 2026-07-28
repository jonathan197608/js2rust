// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 8

function testEdge_frag_8() {
    try {
    const arr = [10, 20, 30];
    const results = [];
    arr.forEach((val, idx) => {
        arr.forEach((val2, idx2) => {
            if (idx !== idx2) {
                results.push(val - val2);
            }
        });
    });
    console.log(results.join(","));
    } catch (e) {
        console.error(`[testEdge_frag_8] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_8();
}

module.exports = { testEdge_frag_8 };
