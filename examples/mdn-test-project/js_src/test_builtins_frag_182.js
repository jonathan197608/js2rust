// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 182
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_182.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_182() {

        // High-low pair OK
        encodeURIComponent("\uD800\uDFFF"); // "%F0%90%8F%BF"

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURIComponent("\uD800");

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURIComponent("\uDFFF");
    }
