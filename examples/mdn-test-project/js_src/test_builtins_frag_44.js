// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 44
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_44.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_44() {

        function isValidIdentifier(str) {
          return /^[$_\p{ID_Start}][$_\p{ID_Continue}]*$/u.test(str);
        }

        isValidIdentifier("foo"); // true
        isValidIdentifier("$1"); // true
        isValidIdentifier("1foo"); // false
        isValidIdentifier("  foo  "); // false
    }
