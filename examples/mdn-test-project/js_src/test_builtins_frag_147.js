// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 147
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_147.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_147() {

        [..."abc".matchAll(/./g)]; // [[ "a" ], [ "b" ], [ "c" ]]
        "abc".replaceAll(/./g, "f"); // "fff"

        const existingPattern = /./;
        const newPattern = new RegExp(
          existingPattern.source,
          `${existingPattern.flags}g`,
        );
        "abc".replaceAll(newPattern, "f"); // "fff"
    }
