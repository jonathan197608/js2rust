// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 70-79)
// Generated: 2026-06-28

function test_expressions_part8() {
    let x = 3;
    let y = 5;
// ---- fragment 70 ----
    try {{
        console.log(5 <= 3);

        console.log(3 <= 3);

        // Compare bigint to number
        console.log(3n <= 5);

        console.log("aa" <= "ab");
    }} catch (e) {{
        console.error(`[test_expressions_part8] fragment 70 error: ${e.message}`);
    }}

    
// ---- fragment 71 ----
    try {{
        x <= y
    }} catch (e) {{
        console.error(`[test_expressions_part8] fragment 71 error: ${e.message}`);
    }}

    
// ---- fragment 72 ----
    try {{
        "a" <= "b"; // true
        "a" <= "a"; // true
        "a" <= "3"; // false
    }} catch (e) {{
        console.error(`[test_expressions_part8] fragment 72 error: ${e.message}`);
    }}

    
// ---- fragment 73 ----
    try {{
        "5" <= 3; // false
        "3" <= 3; // true
        "3" <= 5; // true

        "hello" <= 5; // false
        5 <= "hello"; // false
    }} catch (e) {{
        console.error(`[test_expressions_part8] fragment 73 error: ${e.message}`);
    }}

    
// ---- fragment 74 ----
    try {{
        5 <= 3; // false
        3 <= 3; // true
        3 <= 5; // true
    }} catch (e) {{
        console.error(`[test_expressions_part8] fragment 74 error: ${e.message}`);
    }}

    
// ---- fragment 75 ----
    try {{
        // Mixed BigInt comparisons omitted (TypeError at runtime)
    }} catch (e) {{
        console.error(`[test_expressions_part8] fragment 75 error: ${e.message}`);
    }}

    
// ---- fragment 76 ----
    try {{
        true <= false; // false
        true <= true; // true
        false <= true; // true

        true <= 0; // false
        true <= 1; // true

        null <= 0; // true
        1 <= null; // false

        undefined <= 3; // false
        3 <= undefined; // false

        3 <= NaN; // false
        NaN <= 3; // false
    }} catch (e) {{
        console.error(`[test_expressions_part8] fragment 76 error: ${e.message}`);
    }}

    
// ---- fragment 77 ----
    try {{
        console.log(5 >= 3);

        console.log(3 >= 3);

        // Compare bigint to number
        console.log(3n >= 5);

        console.log("ab" >= "aa");
    }} catch (e) {{
        console.error(`[test_expressions_part8] fragment 77 error: ${e.message}`);
    }}

    
// ---- fragment 78 ----
    try {{
        x >= y
    }} catch (e) {{
        console.error(`[test_expressions_part8] fragment 78 error: ${e.message}`);
    }}

    
// ---- fragment 79 ----
    try {{
        "a" >= "b"; // false
        "a" >= "a"; // true
        "a" >= "3"; // true
    }} catch (e) {{
        console.error(`[test_expressions_part8] fragment 79 error: ${e.message}`);
    }}

    
}
module.exports = { test_expressions_part8 };
