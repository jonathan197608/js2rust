// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 45
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_45.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_45() {

        const variables = ["foo", "foo:bar", "  foo  "];

        function toAssignment(key) {
          if (isValidIdentifier(key)) {
            return `globalThis.${key} = undefined;`;
          }
          // JSON.stringify() escapes quotes and other special characters
          return `globalThis[${JSON.stringify(key)}] = undefined;`;
        }

        const statements = variables.map(toAssignment).join("\n");

        console.log(statements);
        // globalThis.foo = undefined;
        // globalThis["foo:bar"] = undefined;
        // globalThis["  foo  "] = undefined;
    }
