// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 222
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_222.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_222() {

        const replacer = (key, value) =>
          typeof value === "bigint" ? { $bigint: value.toString() } : value;

        const data = {
          number: 1,
          big: 18014398509481982n,
        };
        const stringified = JSON.stringify(data, replacer);

        console.log(stringified);
        // {"number":1,"big":{"$bigint":"18014398509481982"}}
    }
