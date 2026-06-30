// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 2 (fragment 42-43)
// Generated: 2026-06-30

function test_statements_part5() {
// ---- fragment 42 ----
try {{
        let a, x, y;
        const r = 10;

        {
          const { PI, cos, sin } = Math;
          a = PI * r * r;
          x = r * cos(PI);
          y = r * sin(PI / 2);
        }
    }} catch (e) {{
        console.error(`[test_statements_part5] fragment 42 error: ${e.message}`);
    }}

// ---- fragment 43 ----
try {{
        const objectHavingAnEspeciallyLengthyName = { foo: true, bar: false };

        if (((o) => o.foo && !o.bar)(objectHavingAnEspeciallyLengthyName)) {
          // This branch runs.
        }
    }} catch (e) {{
        console.error(`[test_statements_part5] fragment 43 error: ${e.message}`);
    }}

}
module.exports = { testStatements };

}
module.exports = { test_statements_part5 };
