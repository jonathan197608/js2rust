// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 21

function testEdge_frag_21() {
    try {
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
    } catch (e) {
        console.error(`[testEdge_frag_21] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_21();
}

module.exports = { testEdge_frag_21 };
