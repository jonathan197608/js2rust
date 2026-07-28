// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 29
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_29() {
    console.log("2024-01-15".replace(/-(\d{2})/g, "/$1"));
    console.log("hello world".replace(/(\w+)\s(\w+)/, "$2 $1"));
    console.log("test".replace(/t/g, "T"));
    console.log("aaa".replace(/a/g, "$&$&"));
    console.log("hello".replace(/l/g, "L").replace(/L/g, "1"));
    const parts = "a,b;c,d".split(/[,;]/);
    console.log(parts.join("|"));
}
