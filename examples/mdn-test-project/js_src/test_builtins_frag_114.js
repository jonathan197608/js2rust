// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 114
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_114.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_114() {

        parseFloat(3.14);
        parseFloat("3.14");
        parseFloat("  3.14  ");
        parseFloat("314e-2");
        parseFloat("0.0314E+2");
        parseFloat("3.14some non-digit characters");
        parseFloat({
          toString() {
            return "3.14";
          },
        });
    }
