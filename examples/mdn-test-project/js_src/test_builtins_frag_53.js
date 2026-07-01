// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 53
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_53.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_53() {

        function countParagraphs(str) {
          return str.match(/(?:\r?\n){2,}/g).length + 1;
        }

        countParagraphs(`
        Paragraph 1

        Paragraph 2
        Containing some line breaks, but still the same paragraph

        Another paragraph
        `); // 3
    }
