// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 35
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_35.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_35() {

        function getLineTerminators(str) {
          return str.match(/[\r\n\u2028\u2029\q{\r\n}]/gv);
        }

        getLineTerminators(`
        A poem\r
        Is split\r\n
        Into many
        Stanzas
        `); // [ '\r', '\r\n', '\n' ]
    }
