// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 14

function testEdge_frag_14() {
    try {
    console.log("a,b,,c".split(",").length);
    console.log("a,b,,c".split(",").join("|"));
    console.log("hello world".replace("o", "0"));
    console.log("hello world".replaceAll("o", "0"));
    console.log("  trim  ".trim());
    console.log("  trim  ".trimStart().length);
    console.log("hello".repeat(3));
    console.log("abc".padEnd(6, "xyz"));
    } catch (e) {
        console.error(`[testEdge_frag_14] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_14();
}

module.exports = { testEdge_frag_14 };
