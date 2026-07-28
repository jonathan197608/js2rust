// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 21
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_21() {
    function makeMultiplier(factor) {
        return function(arr) {
            return arr.map(function(x) {
                return x * factor;
            }).filter(function(x) {
                return x > factor;
            });
        };
    }
    const double = makeMultiplier(2);
    const triple = makeMultiplier(3);
    console.log(double([1, 2, 3, 4, 5]).join(","));
    console.log(triple([1, 2, 3, 4, 5]).join(","));
}
