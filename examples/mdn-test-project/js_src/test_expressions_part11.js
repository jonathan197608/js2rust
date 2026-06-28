// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 100-109)
// Generated: 2026-06-28

function test_expressions_part11() {
// ---- fragment 100 ----
    try {{
        3 !== "3"; // true
    }} catch (e) {{
        console.error(`[test_expressions_part11] fragment 100 error: ${e.message}`);
    }}

    
// ---- fragment 101 ----
    try {{
        "hello" !== "hello"; // false
        "hello" !== "hola"; // true

        3 !== 3; // false
        3 !== 4; // true

        true !== true; // false
        true !== false; // true

        null !== null; // false
    }} catch (e) {{
        console.error(`[test_expressions_part11] fragment 101 error: ${e.message}`);
    }}

    
// ---- fragment 102 ----
    try {{
        "3" !== 3; // true
        true !== 1; // true
        null !== undefined; // true
    }} catch (e) {{
        console.error(`[test_expressions_part11] fragment 102 error: ${e.message}`);
    }}

    
// ---- fragment 103 ----
    try {{
        const object1 = {
          key: "value",
        };

        const object2 = {
          key: "value",
        };

        console.log(object1 !== object2); // true
        console.log(object1 !== object1); // false
    }} catch (e) {{
        console.error(`[test_expressions_part11] fragment 103 error: ${e.message}`);
    }}

    
// ---- fragment 104 ----
    try {{
        const a = 5; // 00000000000000000000000000000101
        const b = 2; // 00000000000000000000000000000010

        console.log(a << b); // 00000000000000000000000000010100
    }} catch (e) {{
        console.error(`[test_expressions_part11] fragment 104 error: ${e.message}`);
    }}

    
// ---- fragment 105 ----
    try {{
        x << y
    }} catch (e) {{
        console.error(`[test_expressions_part11] fragment 105 error: ${e.message}`);
    }}

    
// ---- fragment 106 ----
    try {{
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[test_expressions_part11] fragment 106 error: ${e.message}`);
    }}

    
// ---- fragment 107 ----
    try {{
        9 << 3; // 72

        // 9 * (2 ** 3) = 9 * (8) = 72

        9n << 3n; // 72n
    }} catch (e) {{
        console.error(`[test_expressions_part11] fragment 107 error: ${e.message}`);
    }}

    
// ---- fragment 108 ----
    try {{
        const a = 5; //  00000000000000000000000000000101
        const b = 2; //  00000000000000000000000000000010
        const c = -5; //  11111111111111111111111111111011

        console.log(a >> b); //  00000000000000000000000000000001

        console.log(c >> b); //  11111111111111111111111111111110
    }} catch (e) {{
        console.error(`[test_expressions_part11] fragment 108 error: ${e.message}`);
    }}

    
// ---- fragment 109 ----
    try {{
        x >> y
    }} catch (e) {{
        console.error(`[test_expressions_part11] fragment 109 error: ${e.message}`);
    }}

    
}
module.exports = { test_expressions_part11 };
