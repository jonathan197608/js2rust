// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 30

function testEdge_frag_30() {
    try {
        const s = "hello";
        console.log(s.concat(" world"));
        const r1 = s.concat(" world");
        console.log(r1);
        console.log(s.at(2));
        const r2 = s.at(2);
        console.log(r2);
        console.log(s.slice(1, 3));
        const r3 = s.slice(1, 3);
        console.log(r3);
        console.log(s.includes("ell"));
        const r4 = s.includes("ell");
        console.log(r4);
    } catch (e) {
        console.error(`[testEdge_frag_30] error: ${e.message}`);
    }
}

if (require.main === module) {
    testEdge_frag_30();
}

module.exports = { testEdge_frag_30 };