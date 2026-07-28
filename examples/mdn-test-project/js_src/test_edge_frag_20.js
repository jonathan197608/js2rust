// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 20
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_20() {
    const { a = 1, b = 2, c = 3 } = { a: 10, c: 30 };
    console.log(a, b, c);
    const [first, second = "def", ...rest] = ["one", , "three", "four"];
    console.log(first);
    console.log(second);
    console.log(rest.join(","));
    const { x: { y } } = { x: { y: 42 } };
    console.log(y);
}
