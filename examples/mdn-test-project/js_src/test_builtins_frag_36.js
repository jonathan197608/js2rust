// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 36
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_36.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_36() {

                const s = 1;
function splitWords(str) {
          return str.split(/\s+/);
        }

        splitWords(`Look at the stars
        Look  how they\tshine for you`);
        // ['Look', 'at', 'the', 'stars', 'Look', 'how', 'they', 'shine', 'for', 'you']
    }
