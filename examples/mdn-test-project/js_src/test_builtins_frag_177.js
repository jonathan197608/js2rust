// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 177
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_177.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_177() {

        // High-low pair OK
        encodeURI("\uD800\uDFFF"); // "%F0%90%8F%BF"

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURI("\uD800");

        // Lone low-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURI("\uDFFF");
    }
