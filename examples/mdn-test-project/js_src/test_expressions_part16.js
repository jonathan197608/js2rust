// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 150-159)
// Generated: 2026-06-28

function test_expressions_part16() {
    var bCondition1 = true;
    var bCondition2 = true;
    var bCondition3 = true;
    var x = 0;
    var y = 0;
// ---- fragment 150 ----
    try {{
        bCondition1 && (bCondition2 || bCondition3)
    }} catch (e) {{
        console.error(`[test_expressions_part16] fragment 150 error: ${e.message}`);
    }}

    
// ---- fragment 151 ----
    try {{
        !(!bCondition1 || !bCondition2 && !bCondition3)
    }} catch (e) {{
        console.error(`[test_expressions_part16] fragment 151 error: ${e.message}`);
    }}

    
// ---- fragment 152 ----
    try {{
        let a = 3;

        console.log((a %= 2));

        console.log((a %= 0));

        console.log((a %= "hello"));
    }} catch (e) {{
        console.error(`[test_expressions_part16] fragment 152 error: ${e.message}`);
    }}

    
// ---- fragment 153 ----
    try {{
        x %= y
    }} catch (e) {{
        console.error(`[test_expressions_part16] fragment 153 error: ${e.message}`);
    }}

    
// ---- fragment 154 ----
    try {{
        let bar = 5;

        bar %= 2; // 1
        bar %= "foo"; // NaN
        bar %= 0; // NaN

        let foo = 3n;
        foo %= 2n; // 1n
    }} catch (e) {{
        console.error(`[test_expressions_part16] fragment 154 error: ${e.message}`);
    }}

    
// ---- fragment 155 ----
    try {{
        let a = 2;

        console.log((a -= 3));

        console.log((a -= "Hello"));
    }} catch (e) {{
        console.error(`[test_expressions_part16] fragment 155 error: ${e.message}`);
    }}

    
// ---- fragment 156 ----
    try {{
        x -= y
    }} catch (e) {{
        console.error(`[test_expressions_part16] fragment 156 error: ${e.message}`);
    }}

    
// ---- fragment 157 ----
    try {{
        let bar = 5;

        bar -= 2; // 3
    }} catch (e) {{
        console.error(`[test_expressions_part16] fragment 157 error: ${e.message}`);
    }}

    
// ---- fragment 158 ----
    try {{
        let bar = 0;
        bar -= "foo"; // NaN
    }} catch (e) {{
        console.error(`[test_expressions_part16] fragment 158 error: ${e.message}`);
    }}

    
// ---- fragment 159 ----
    try {{
        let foo = 3n;
        foo -= 2n; // 1n
        foo -= 1; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[test_expressions_part16] fragment 159 error: ${e.message}`);
    }}

    
}
module.exports = { test_expressions_part16 };
