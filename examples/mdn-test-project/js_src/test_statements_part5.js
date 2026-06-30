// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 4 (fragment 40-43)
// Generated: 2026-06-28

function test_statements_part5() {
// ---- fragment 40 ----
    try {{
        const foo = { bar: 1 };
        console.log(foo.bar);
        // foo is found in the scope chain as a variable;
        // bar is found in foo as a property
    }} catch (e) {{
        console.error(`[test_statements_part5] fragment 40 error: ${e.message}`);
    }}

    
// ---- fragment 41 ----
    try {{
        console.log(1 === 1); // true
    }} catch (e) {{
        console.error(`[test_statements_part5] fragment 41 error: ${e.message}`);
    }}

    
// ---- fragment 42 ----
    try {{
        let a = 0, x = 0, y = 0;
        const r = 10;

        {
          const PI = 3.14159;
          a = PI * r * r;
          x = r * (-1);
          y = r * 1;
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
module.exports = { test_statements_part5 };
