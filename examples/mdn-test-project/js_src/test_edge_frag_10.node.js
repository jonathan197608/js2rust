// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 10

function testEdge_frag_10() {
    try {
    const data = [
        { name: "Alice", age: 30 },
        { name: "Bob", age: 25 },
        { name: "Charlie", age: 35 }
    ];
    const found = data.find(p => p.age > 28);
    console.log(found.name);
    const idx = data.findIndex(p => p.name === "Bob");
    console.log(idx);
    const lastFound = data.findLast(p => p.age < 35);
    console.log(lastFound.name);
    console.log(data.some(p => p.age > 30));
    console.log(data.every(p => p.age > 20));
    } catch (e) {
        console.error(`[testEdge_frag_10] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_10();
}

module.exports = { testEdge_frag_10 };
