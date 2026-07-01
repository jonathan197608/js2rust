// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 219
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_219.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_219() {

        if (0n) {
          console.log("Hello from the if!");
        } else {
          console.log("Hello from the else!");
        }
        // "Hello from the else!"

        0n || 12n; // 12n
        0n && 12n; // 0n
        Boolean(0n); // false
        Boolean(12n); // true
        !12n; // false
        !0n; // true
    }
