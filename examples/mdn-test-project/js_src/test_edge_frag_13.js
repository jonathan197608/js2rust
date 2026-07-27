// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 13
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_13() {
    console.log([1,2,3].at(-1));
    console.log([1,2,3].at(-4));
    console.log([1,2,3].includes(2));
    console.log([1,2,3].includes(2, 2));
    console.log([1,2,3].indexOf(2, -1));
    console.log([1,2,3].slice(-2));
    console.log([1,2,3].concat([4,5], [6]));
    console.log([1,2,3].fill(0, 1, 2).join(","));
    console.log([1,2,3,4,5].with(2, 99).join(","));
}
