// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 20-29)
// Generated: 2026-06-30

function test_expressions_part3() {
// ---- fragment 20 ----
try {{
        2n ** 3n; // 8n
        2n ** 1024n; // A very large number, but not Infinity
    }} catch (e) {{
        console.error(`[test_expressions_part3] fragment 20 error: ${e.message}`);
    }}

// ---- fragment 21 ----
try {{
        2n ** 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 ** 2n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[test_expressions_part3] fragment 21 error: ${e.message}`);
    }}

// ---- fragment 22 ----
try {{
        2n ** BigInt(2); // 4n
        Number(2n) ** 2; // 4
    }} catch (e) {{
        console.error(`[test_expressions_part3] fragment 22 error: ${e.message}`);
    }}

// ---- fragment 23 ----
try {{
        2 ** 3 ** 2; // 512
        2 ** (3 ** 2); // 512
        (2 ** 3) ** 2; // 64
    }} catch (e) {{
        console.error(`[test_expressions_part3] fragment 23 error: ${e.message}`);
    }}

// ---- fragment 24 ----
try {{
        -(2 ** 2); // -4
    }} catch (e) {{
        console.error(`[test_expressions_part3] fragment 24 error: ${e.message}`);
    }}

// ---- fragment 25 ----
try {{
        (-2) ** 2; // 4
    }} catch (e) {{
        console.error(`[test_expressions_part3] fragment 25 error: ${e.message}`);
    }}

// ---- fragment 26 ----
try {{
        console.log(3 * 4);

        console.log(-3 * 4);

        console.log("3" * 2);

        console.log("foo" * 2);
    }} catch (e) {{
        console.error(`[test_expressions_part3] fragment 26 error: ${e.message}`);
    }}

// ---- fragment 27 ----
try {{
        var x = 1;
        var y = 2;
        x * y
    }} catch (e) {{
        console.error(`[test_expressions_part3] fragment 27 error: ${e.message}`);
    }}

// ---- fragment 28 ----
try {{
        2 * 2; // 4
        -2 * 2; // -4

        Infinity * 0; // NaN
        Infinity * Infinity; // Infinity
    }} catch (e) {{
        console.error(`[test_expressions_part3] fragment 28 error: ${e.message}`);
    }}

// ---- fragment 29 ----
try {{
        "foo" * 2; // NaN
        "2" * 2; // 4
    }} catch (e) {{
        console.error(`[test_expressions_part3] fragment 29 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part3 };
