// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 8 (fragment 160-167)
// Generated: 2026-06-28

function test_expressions_part17() {
// ---- fragment 160 ----
    try {{
        let a = 3;

        console.log((a **= 2));

        console.log((a **= 0));

        console.log((a **= 'hello'));
    }} catch (e) {{
        console.error(`[test_expressions_part17] fragment 160 error: ${e.message}`);
    }}

    
// ---- fragment 161 ----
    try {{
        x **= y
    }} catch (e) {{
        console.error(`[test_expressions_part17] fragment 161 error: ${e.message}`);
    }}

    
// ---- fragment 162 ----
    try {{
        let bar = 5;
        bar **= 2; // 25
    }} catch (e) {{
        console.error(`[test_expressions_part17] fragment 162 error: ${e.message}`);
    }}

    
// ---- fragment 163 ----
    try {{
        let baz = 5;
        baz **= "foo"; // NaN
    }} catch (e) {{
        console.error(`[test_expressions_part17] fragment 163 error: ${e.message}`);
    }}

    
// ---- fragment 164 ----
    try {{
        let foo = 3n;
        foo **= 2n; // 9n
        foo **= 1; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[test_expressions_part17] fragment 164 error: ${e.message}`);
    }}

    
// ---- fragment 165 ----
    try {{
        function getVowels(str) {
          const m = str.match(/[aeiou]/gi);
          if (m === null) {
            return 0;
          }
          return m.length;
        }

        console.log(getVowels("sky"));
    }} catch (e) {{
        console.error(`[test_expressions_part17] fragment 165 error: ${e.message}`);
    }}

    
// ---- fragment 166 ----
    try {{
        null
    }} catch (e) {{
        console.error(`[test_expressions_part17] fragment 166 error: ${e.message}`);
    }}

    
// ---- fragment 167 ----
    try {{
        typeof null; // "object" (not "null" for legacy reasons)
        typeof undefined; // "undefined"
        null === undefined; // false
        null == undefined; // true
        null === null; // true
        null == null; // true
        !null; // true
        Number.isNaN(1 + null); // false
        Number.isNaN(1 + undefined); // true
    }} catch (e) {{
        console.error(`[test_expressions_part17] fragment 167 error: ${e.message}`);
    }}

}
module.exports = { testExpressions };
module.exports = { test_expressions_part17 };
