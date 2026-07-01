// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 40-49)
// Generated: 2026-06-28

function test_expressions_part5() {
// ---- fragment 40 ----
    try {{
        console.log(13 % 5);

        console.log(-13 % 5);

        console.log(4 % 2);

        console.log(-4 % 2);
    }} catch (e) {{
        console.error(`[test_expressions_part5] fragment 40 error: ${e.message}`);
    }}

    
// ---- fragment 41 ----
    try {{
        x % y
    }} catch (e) {{
        console.error(`[test_expressions_part5] fragment 41 error: ${e.message}`);
    }}

    
// ---- fragment 42 ----
    try {{
        13 % 5; // 3
        1 % -2; // 1
        1 % 2; // 1
        2 % 3; // 2
        5.5 % 2; // 1.5

        3n % 2n; // 1n
    }} catch (e) {{
        console.error(`[test_expressions_part5] fragment 42 error: ${e.message}`);
    }}

    
// ---- fragment 43 ----
    try {{
        -13 % 5; // -3
        -1 % 2; // -1
        -4 % 2; // -0

        -3n % 2n; // -1n
    }} catch (e) {{
        console.error(`[test_expressions_part5] fragment 43 error: ${e.message}`);
    }}

    
// ---- fragment 44 ----
    try {{
        NaN % 2; // NaN
    }} catch (e) {{
        console.error(`[test_expressions_part5] fragment 44 error: ${e.message}`);
    }}

    
// ---- fragment 45 ----
    try {{
        Infinity % 2; // NaN
        Infinity % 0; // NaN
        Infinity % Infinity; // NaN
        2 % Infinity; // 2
        0 % Infinity; // 0
    }} catch (e) {{
        console.error(`[test_expressions_part5] fragment 45 error: ${e.message}`);
    }}

    
// ---- fragment 46 ----
    try {{
        console.log(2 + 2);

        console.log(2 + true);

        console.log("hello " + "everyone");

        console.log(2001 + ": A Space Odyssey");
    }} catch (e) {{
        console.error(`[test_expressions_part5] fragment 46 error: ${e.message}`);
    }}

    
// ---- fragment 47 ----
    try {{
        x + y
    }} catch (e) {{
        console.error(`[test_expressions_part5] fragment 47 error: ${e.message}`);
    }}

    
// ---- fragment 48 ----
    try {{
        const t = Temporal.Now.instant();
        "" + t; // Throws TypeError
        `${t}`; // '2022-07-31T04:48:56.113918308Z'
        "".concat(t); // '2022-07-31T04:48:56.113918308Z'
    }} catch (e) {{
        console.error(`[test_expressions_part5] fragment 48 error: ${e.message}`);
    }}

    
// ---- fragment 49 ----
    try {{
        1 + 2; // 3
    }} catch (e) {{
        console.error(`[test_expressions_part5] fragment 49 error: ${e.message}`);
    }}

    
}
module.exports = { test_expressions_part5 };
