// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 0
// Run with Node.js: node test_statements_frag_0.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_0() {
    function getRectArea(width, height) {
      if (width > 0 && height > 0) {
        return width * height;
      }
      return 0;
    }

    console.log(getRectArea(3, 4));

    console.log(getRectArea(-3, 4));
}
