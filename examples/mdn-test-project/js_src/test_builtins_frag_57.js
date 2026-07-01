// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 57
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_57.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_57() {

        function hasThanks(str) {
          return /\b(thanks|thank you)\b/i.test(str);
        }

        hasThanks("Thanks! You helped me a lot."); // true
        hasThanks("Just want to say thank you for all your work."); // true
        hasThanks("Thanksgiving is around the corner."); // false
    }
