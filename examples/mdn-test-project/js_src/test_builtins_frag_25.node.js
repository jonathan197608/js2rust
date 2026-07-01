// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 25
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_25.node.js

function testBuiltins_frag_25() {
    try {

        const littleEndian = (() => {
          const buffer = new ArrayBuffer(2);
          new DataView(buffer).setInt16(0, 256, true /* littleEndian */);
          // Int16Array uses the platform's endianness.
          return new Int16Array(buffer)[0] === 256;
        })();
        console.log(littleEndian); // true or false
        } catch (e) {
        console.error(`[testBuiltins_frag_25] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_25();
}

module.exports = { testBuiltins_frag_25 };
