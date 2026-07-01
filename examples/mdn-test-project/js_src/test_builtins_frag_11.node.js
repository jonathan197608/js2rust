// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 11
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_11.node.js

function testBuiltins_frag_11() {
    try {

        function degToRad(degrees) {
          return degrees * (Math.PI / 180);
        }

        function radToDeg(rad) {
          return rad / (Math.PI / 180);
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_11] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_11();
}

module.exports = { testBuiltins_frag_11 };
