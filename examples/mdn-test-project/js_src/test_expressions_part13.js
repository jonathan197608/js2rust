// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 120-129)
// Generated: 2026-06-30

function test_expressions_part13() {
// ---- fragment 120 ----
try {{
        // 9  (00000000000000000000000000001001)
        // 14 (00000000000000000000000000001110)

        14 & 9;
        // 8  (00000000000000000000000000001000)

        14n & 9n; // 8n
    }} catch (e) {{
        console.error(`[test_expressions_part13] fragment 120 error: ${e.message}`);
    }}

// ---- fragment 121 ----
try {{
        const a = 5; // 00000000000000000000000000000101
        const b = 3; // 00000000000000000000000000000011

        console.log(a | b); // 00000000000000000000000000000111
    }} catch (e) {{
        console.error(`[test_expressions_part13] fragment 121 error: ${e.message}`);
    }}

// ---- fragment 122 ----
try {{
        var x = 1;
        var y = 2;
        x | y
    }} catch (e) {{
        console.error(`[test_expressions_part13] fragment 122 error: ${e.message}`);
    }}

// ---- fragment 123 ----
try {{
        var After = 0;
        var Before = 0;
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
            _ = After;
        _ = Before;
}} catch (e) {{
        console.error(`[test_expressions_part13] fragment 123 error: ${e.message}`);
    }}

// ---- fragment 124 ----
try {{
        // 9  (00000000000000000000000000001001)
        // 14 (00000000000000000000000000001110)

        14 | 9;
        // 15 (00000000000000000000000000001111)

        14n | 9n; // 15n
    }} catch (e) {{
        console.error(`[test_expressions_part13] fragment 124 error: ${e.message}`);
    }}

// ---- fragment 125 ----
try {{
        const a = 5; // 00000000000000000000000000000101
        const b = 3; // 00000000000000000000000000000011

        console.log(a ^ b); // 00000000000000000000000000000110
    }} catch (e) {{
        console.error(`[test_expressions_part13] fragment 125 error: ${e.message}`);
    }}

// ---- fragment 126 ----
try {{
        var x = 1;
        var y = 2;
        x ^ y
    }} catch (e) {{
        console.error(`[test_expressions_part13] fragment 126 error: ${e.message}`);
    }}

// ---- fragment 127 ----
try {{
        var After = 0;
        var Before = 0;
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
            _ = After;
        _ = Before;
}} catch (e) {{
        console.error(`[test_expressions_part13] fragment 127 error: ${e.message}`);
    }}

// ---- fragment 128 ----
try {{
        // 9  (00000000000000000000000000001001)
        // 14 (00000000000000000000000000001110)

        14 ^ 9;
        // 7  (00000000000000000000000000000111)

        14n ^ 9n; // 7n
    }} catch (e) {{
        console.error(`[test_expressions_part13] fragment 128 error: ${e.message}`);
    }}

// ---- fragment 129 ----
try {{
        const a = 3;
        const b = -2;

        console.log(a > 0 && b > 0);
    }} catch (e) {{
        console.error(`[test_expressions_part13] fragment 129 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part13 };
