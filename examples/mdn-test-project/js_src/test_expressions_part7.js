// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 60-69)
// Generated: 2026-06-30

function test_expressions_part7() {
// ---- fragment 60 ----
try {{
        2n - 1n; // 1n
    }} catch (e) {{
        console.error(`[test_expressions_part7] fragment 60 error: ${e.message}`);
    }}

// ---- fragment 61 ----
try {{
        2n - 1; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 - 1n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[test_expressions_part7] fragment 61 error: ${e.message}`);
    }}

// ---- fragment 62 ----
try {{
        2n - BigInt(1); // 1n
        Number(2n) - 1; // 1
    }} catch (e) {{
        console.error(`[test_expressions_part7] fragment 62 error: ${e.message}`);
    }}

// ---- fragment 63 ----
try {{
        console.log(5 > 3);

        console.log(3 > 3);

        // Compare bigint to number
        console.log(3n > 5);

        console.log("ab" > "aa");
    }} catch (e) {{
        console.error(`[test_expressions_part7] fragment 63 error: ${e.message}`);
    }}

// ---- fragment 64 ----
try {{
        x > y
    }} catch (e) {{
        console.error(`[test_expressions_part7] fragment 64 error: ${e.message}`);
    }}

// ---- fragment 65 ----
try {{
        "a" > "b"; // false
        "a" > "a"; // false
        "a" > "3"; // true
    }} catch (e) {{
        console.error(`[test_expressions_part7] fragment 65 error: ${e.message}`);
    }}

// ---- fragment 66 ----
try {{
        "5" > 3; // true
        "3" > 3; // false
        "3" > 5; // false

        "hello" > 5; // false
        5 > "hello"; // false

        "5" > 3n; // true
        "3" > 5n; // false
    }} catch (e) {{
        console.error(`[test_expressions_part7] fragment 66 error: ${e.message}`);
    }}

// ---- fragment 67 ----
try {{
        5 > 3; // true
        3 > 3; // false
        3 > 5; // false
    }} catch (e) {{
        console.error(`[test_expressions_part7] fragment 67 error: ${e.message}`);
    }}

// ---- fragment 68 ----
try {{
        5n > 3; // true
        3 > 5n; // false
    }} catch (e) {{
        console.error(`[test_expressions_part7] fragment 68 error: ${e.message}`);
    }}

// ---- fragment 69 ----
try {{
        true > false; // true
        false > true; // false

        true > 0; // true
        true > 1; // false

        null > 0; // false
        1 > null; // true

        undefined > 3; // false
        3 > undefined; // false

        3 > NaN; // false
        NaN > 3; // false
    }} catch (e) {{
        console.error(`[test_expressions_part7] fragment 69 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part7 };
