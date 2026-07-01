// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 43
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_43.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_43() {

        function isImage(filename) {
          return /\.(?:png|jpe?g|webp|avif|gif)$/i.test(filename);
        }

        isImage("image.png"); // true
        isImage("image.jpg"); // true
        isImage("image.pdf"); // false
    }
