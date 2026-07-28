// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 9
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_9() {
    const nums = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5];
    const sortedAsc = [...nums].sort((a, b) => a - b);
    const sortedDesc = [...nums].sort((a, b) => b - a);
    console.log(sortedAsc.join(","));
    console.log(sortedDesc.join(","));
    const strs = ["banana", "apple", "cherry"];
    strs.sort();
    console.log(strs.join(","));
}
