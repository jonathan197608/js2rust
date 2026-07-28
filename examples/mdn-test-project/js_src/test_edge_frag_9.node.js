// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 9

function testEdge_frag_9() {
    try {
    const nums = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5];
    const sortedAsc = [...nums].sort((a, b) => a - b);
    const sortedDesc = [...nums].sort((a, b) => b - a);
    console.log(sortedAsc.join(","));
    console.log(sortedDesc.join(","));
    const strs = ["banana", "apple", "cherry"];
    strs.sort();
    console.log(strs.join(","));
    } catch (e) {
        console.error(`[testEdge_frag_9] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_9();
}

module.exports = { testEdge_frag_9 };
