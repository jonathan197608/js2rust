// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 174
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_174.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_174() {

                const g = 1;
function decodeQueryParam(p) {
          return decodeURIComponent(p.replace(/\+/g, " "));
        }

        decodeQueryParam("search+query%20%28correct%29");
        // 'search query (correct)'
    }
