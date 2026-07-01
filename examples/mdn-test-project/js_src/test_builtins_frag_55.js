// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 55
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_55.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_55() {

        // Ù¢ is the digit 2 in Arabic-Indic notation
        // while it is predominantly written within the Arabic script
        // it can also be written in the Thaana script

        "Ù¢".match(/\p{Script=Thaana}/u);
        // null as Thaana is not the predominant script

        "Ù¢".match(/\p{Script_Extensions=Thaana}/u);
        // ["Ù¢", index: 0, input: "Ù¢", groups: undefined]
    }
