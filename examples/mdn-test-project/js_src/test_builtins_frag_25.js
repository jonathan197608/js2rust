// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 25
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_25.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_25() {

        const littleEndian = (() => {
          const buffer = new ArrayBuffer(2);
          new DataView(buffer).setInt16(0, 256, true /* littleEndian */);
          // Int16Array uses the platform's endianness.
          return new Int16Array(buffer)[0] === 256;
        })();
        console.log(littleEndian); // true or false
    }
