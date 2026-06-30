// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 50-59)
// Generated: 2026-06-30

function test_expressions_part6() {
// ---- fragment 50 ----
try {{
        true + 1; // 2
        false + false; // 0
    }} catch (e) {{
        console.error(`[test_expressions_part6] fragment 50 error: ${e.message}`);
    }}

// ---- fragment 51 ----
try {{
        1n + 2n; // 3n
    }} catch (e) {{
        console.error(`[test_expressions_part6] fragment 51 error: ${e.message}`);
    }}

// ---- fragment 52 ----
try {{
        1n + 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 + 1n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[test_expressions_part6] fragment 52 error: ${e.message}`);
    }}

// ---- fragment 53 ----
try {{
        "1" + 2n; // "12"
    }} catch (e) {{
        console.error(`[test_expressions_part6] fragment 53 error: ${e.message}`);
    }}

// ---- fragment 54 ----
try {{
        1n + BigInt(2); // 3n
        Number(1n) + 2; // 3
    }} catch (e) {{
        console.error(`[test_expressions_part6] fragment 54 error: ${e.message}`);
    }}

// ---- fragment 55 ----
try {{
        "foo" + "bar"; // "foobar"
        5 + "foo"; // "5foo"
        "foo" + false; // "foofalse"
        "2" + 2; // "22"
    }} catch (e) {{
        console.error(`[test_expressions_part6] fragment 55 error: ${e.message}`);
    }}

// ---- fragment 56 ----
try {{
        console.log(5 - 3);

        console.log(3.5 - 5);

        console.log(5 - "hello");

        console.log(5 - true);
    }} catch (e) {{
        console.error(`[test_expressions_part6] fragment 56 error: ${e.message}`);
    }}

// ---- fragment 57 ----
try {{
        var x = 1;
        var y = 2;
        x - y
    }} catch (e) {{
        console.error(`[test_expressions_part6] fragment 57 error: ${e.message}`);
    }}

// ---- fragment 58 ----
try {{
        5 - 3; // 2
        3 - 5; // -2
    }} catch (e) {{
        console.error(`[test_expressions_part6] fragment 58 error: ${e.message}`);
    }}

// ---- fragment 59 ----
try {{
        "foo" - 3; // NaN; "foo" is converted to the number NaN
        5 - "3"; // 2; "3" is converted to the number 3
    }} catch (e) {{
        console.error(`[test_expressions_part6] fragment 59 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part6 };
