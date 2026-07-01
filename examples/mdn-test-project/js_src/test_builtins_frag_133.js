// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 133
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_133.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_133() {

        // Only setting the prototype once
        const obj = { __proto__: { a: 1 } };

        // These syntaxes all create a property called "__proto__" and can coexist
        // They would overwrite each other and the last one is actually used
        const __proto__ = null;
        const obj2 = {
          ["__proto__"]: {},
          __proto__,
          __proto__() {},
          get __proto__() {
            return 1;
          },
        };
    }
