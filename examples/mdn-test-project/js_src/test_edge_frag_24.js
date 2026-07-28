// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 24
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_24() {
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
}
