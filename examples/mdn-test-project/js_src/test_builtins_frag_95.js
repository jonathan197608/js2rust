// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 95
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_95.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_95() {
    try {
        JSON.parse("[1, 2, 3, 4,]");
        JSON.parse('{"foo": 1,}');
        // SyntaxError JSON.parse: unexpected character
        // at line 1 column 14 of the JSON data
    } catch (e) {
        // Expected SyntaxError: invalid JSON (trailing comma)
    }
    }
