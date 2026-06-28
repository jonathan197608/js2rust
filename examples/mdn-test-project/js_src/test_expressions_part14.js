// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 130-139)
// Generated: 2026-06-28

function test_expressions_part14() {
// ---- fragment 130 ----
    try {{
        x && y
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 130 error: ${e.message}`);
    }}

    
// ---- fragment 131 ----
    try {{
        result = "" && "foo"; // result is assigned "" (empty string)
        result = 2 && 0; // result is assigned 0
        result = "foo" && 4; // result is assigned 4
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 131 error: ${e.message}`);
    }}

    
// ---- fragment 132 ----
    try {{
        function A() {
          console.log("called A");
          return false;
        }
        function B() {
          console.log("called B");
          return true;
        }

        console.log(A() && B());
        // Logs "called A" to the console due to the call for function A,
        // && evaluates to false (function A returns false), then false is logged to the console;
        // the AND operator short-circuits here and ignores function B
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 132 error: ${e.message}`);
    }}

    
// ---- fragment 133 ----
    try {{
        true || false && false; // true
        true && (false || false); // false
        (2 === 3) || (4 < 0) && (1 === 1); // false
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 133 error: ${e.message}`);
    }}

    
// ---- fragment 134 ----
    try {{
        a1 = true && true; // t && t returns true
        a2 = true && false; // t && f returns false
        a3 = false && true; // f && t returns false
        a4 = false && 3 === 4; // f && f returns false
        a5 = "Cat" && "Dog"; // t && t returns "Dog"
        a6 = false && "Cat"; // f && t returns false
        a7 = "Cat" && false; // t && f returns false
        a8 = "" && false; // f && f returns ""
        a9 = false && ""; // f && f returns false
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 134 error: ${e.message}`);
    }}

    
// ---- fragment 135 ----
    try {{
        bCondition1 && bCondition2
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 135 error: ${e.message}`);
    }}

    
// ---- fragment 136 ----
    try {{
        !(!bCondition1 || !bCondition2)
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 136 error: ${e.message}`);
    }}

    
// ---- fragment 137 ----
    try {{
        bCondition1 || bCondition2
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 137 error: ${e.message}`);
    }}

    
// ---- fragment 138 ----
    try {{
        !(!bCondition1 && !bCondition2)
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 138 error: ${e.message}`);
    }}

    
// ---- fragment 139 ----
    try {{
        bCondition1 || (bCondition2 && bCondition3)
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 139 error: ${e.message}`);
    }}

    
}
module.exports = { test_expressions_part14 };
