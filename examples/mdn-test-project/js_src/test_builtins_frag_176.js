// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 176
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_176.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_176() {

        const set1 = ";/?:@&=+$,#"; // Reserved Characters
        const set2 = "-.!~*'()"; // Unreserved Marks
        const set3 = "ABC abc 123"; // Alphanumeric Characters + Space

        console.log(encodeURI(set1)); // ;/?:@&=+$,#
        console.log(encodeURI(set2)); // -.!~*'()
        console.log(encodeURI(set3)); // ABC%20abc%20123 (the space gets encoded as %20)

        console.log(encodeURIComponent(set1)); // %3B%2C%2F%3F%3A%40%26%3D%2B%24%23
        console.log(encodeURIComponent(set2)); // -.!~*'()
        console.log(encodeURIComponent(set3)); // ABC%20abc%20123 (the space gets encoded as %20)
    }
