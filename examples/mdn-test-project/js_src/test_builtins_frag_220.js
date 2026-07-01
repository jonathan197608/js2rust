// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 220
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_220.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_220() {

        BigInt.prototype.toJSON = function () {
          return { $bigint: this.toString() };
        };
    }
