// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 8
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_8() {
    const arr = [10, 20, 30];
    const results = [];
    arr.forEach((val, idx) => {
        arr.forEach((val2, idx2) => {
            if (idx !== idx2) {
                results.push(val - val2);
            }
        });
    });
    console.log(results.join(","));
}
