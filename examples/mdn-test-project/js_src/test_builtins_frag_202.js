// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 202
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_202.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_202() {

        try {
          decodeURIComponent("%");
        } catch (e) {
          console.log(e instanceof URIError); // true
          console.log(e.message); // "malformed URI sequence"
          console.log(e.name); // "URIError"
          console.log(e.stack); // Stack of the error
        }
    }
