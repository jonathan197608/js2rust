// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 41
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_41.node.js

function testBuiltins_frag_41() {
    try {

        function isImage(filename) {
          return /\.(?:png|jpe?g|webp|avif|gif)$/i.test(filename);
        }

        isImage("image.png"); // true
        isImage("image.jpg"); // true
        isImage("image.pdf"); // false
        } catch (e) {
        console.error(`[testBuiltins_frag_41] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_41();
}

module.exports = { testBuiltins_frag_41 };
