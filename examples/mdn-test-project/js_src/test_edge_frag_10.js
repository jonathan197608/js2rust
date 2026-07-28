// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 10
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_10() {
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
}
