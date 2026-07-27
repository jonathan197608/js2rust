// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 3

function testEdge_frag_3() {
    try {
    const a = null ?? 42;
    const b = undefined ?? "default";
    const c = 0 ?? 100;
    const d = false ?? true;
    const e = "" ?? "empty";
    console.log(a, b, c, d, e);
    console.log(null ?? undefined ?? 0);
    console.log(0 || null || undefined || "last");
    } catch (e) {
        console.error(`[testEdge_frag_3] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_3();
}

module.exports = { testEdge_frag_3 };
