// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 29

function testEdge_frag_29() {
    try {
    console.log("2024-01-15".replace(/-(\d{2})/g, "/$1"));
    console.log("hello world".replace(/(\w+)\s(\w+)/, "$2 $1"));
    console.log("test".replace(/t/g, "T"));
    console.log("aaa".replace(/a/g, "$&$&"));
    console.log("hello".replace(/l/g, "L").replace(/L/g, "1"));
    const parts = "a,b;c,d".split(/[,;]/);
    console.log(parts.join("|"));
    } catch (e) {
        console.error(`[testEdge_frag_29] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_29();
}

module.exports = { testEdge_frag_29 };
