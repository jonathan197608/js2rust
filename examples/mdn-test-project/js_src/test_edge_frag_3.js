// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 3
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_3() {
    const a = null ?? 42;
    const b = undefined ?? "default";
    const c = 0 ?? 100;
    const d = false ?? true;
    const e = "" ?? "empty";
    console.log(a, b, c, d, e);
    console.log(null ?? undefined ?? 0);
    console.log(0 || null || undefined || "last");
}
