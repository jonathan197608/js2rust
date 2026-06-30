// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 30-39)
// Generated: 2026-06-30

function test_expressions_part4() {
// ---- fragment 30 ----
try {{
        2n * 2n; // 4n
        -2n * 2n; // -4n
    }} catch (e) {{
        console.error(`[test_expressions_part4] fragment 30 error: ${e.message}`);
    }}

// ---- fragment 31 ----
try {{
        2n * 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 * 2n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[test_expressions_part4] fragment 31 error: ${e.message}`);
    }}

// ---- fragment 32 ----
try {{
        2n * BigInt(2); // 4n
        Number(2n) * 2; // 4
    }} catch (e) {{
        console.error(`[test_expressions_part4] fragment 32 error: ${e.message}`);
    }}

// ---- fragment 33 ----
try {{
        console.log(12 / 2);

        console.log(3 / 2);

        console.log(6 / "3");

        console.log(2 / 0);
    }} catch (e) {{
        console.error(`[test_expressions_part4] fragment 33 error: ${e.message}`);
    }}

// ---- fragment 34 ----
try {{
        x / y
    }} catch (e) {{
        console.error(`[test_expressions_part4] fragment 34 error: ${e.message}`);
    }}

// ---- fragment 35 ----
try {{
        1 / 2; // 0.5
        Math.floor(3 / 2); // 1
        1.0 / 2.0; // 0.5

        2 / 0; // Infinity
        2.0 / 0.0; // Infinity, because 0.0 === 0
        2.0 / -0.0; // -Infinity
    }} catch (e) {{
        console.error(`[test_expressions_part4] fragment 35 error: ${e.message}`);
    }}

// ---- fragment 36 ----
try {{
        5 / "2"; // 2.5
        5 / "foo"; // NaN
    }} catch (e) {{
        console.error(`[test_expressions_part4] fragment 36 error: ${e.message}`);
    }}

// ---- fragment 37 ----
try {{
        1n / 2n; // 0n
        5n / 3n; // 1n
        -1n / 3n; // 0n
        1n / -3n; // 0n

        2n / 0n; // RangeError: BigInt division by zero
    }} catch (e) {{
        console.error(`[test_expressions_part4] fragment 37 error: ${e.message}`);
    }}

// ---- fragment 38 ----
try {{
        2n / 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 / 2n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[test_expressions_part4] fragment 38 error: ${e.message}`);
    }}

// ---- fragment 39 ----
try {{
        2n / BigInt(2); // 1n
        Number(2n) / 2; // 1
    }} catch (e) {{
        console.error(`[test_expressions_part4] fragment 39 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part4 };
