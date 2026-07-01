// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 140-149)
// Generated: 2026-06-28

function test_expressions_part15() {
// ---- fragment 140 ----
    try {{
        bCondition1 || bCondition2 && bCondition3
    }} catch (e) {{
        console.error(`[test_expressions_part15] fragment 140 error: ${e.message}`);
    }}

    
// ---- fragment 141 ----
    try {{
        const a = 3;
        const b = -2;

        console.log(a > 0 || b > 0);
    }} catch (e) {{
        console.error(`[test_expressions_part15] fragment 141 error: ${e.message}`);
    }}

    
// ---- fragment 142 ----
    try {{
        x || y
    }} catch (e) {{
        console.error(`[test_expressions_part15] fragment 142 error: ${e.message}`);
    }}

    
// ---- fragment 143 ----
    try {{
        function A() {
          console.log("called A");
          return false;
        }
        function B() {
          console.log("called B");
          return true;
        }

        console.log(B() || A());
        // Logs "called B" due to the function call,
        // then logs true (which is the resulting value of the operator)
    }} catch (e) {{
        console.error(`[test_expressions_part15] fragment 143 error: ${e.message}`);
    }}

    
// ---- fragment 144 ----
    try {{
        true || false && false; // returns true, because && is executed first
        (true || false) && false; // returns false, because grouping has the highest precedence
    }} catch (e) {{
        console.error(`[test_expressions_part15] fragment 144 error: ${e.message}`);
    }}

    
// ---- fragment 145 ----
    try {{
        // SKIP: non-bool || operands
    }} catch (e) {{
        console.error(`[test_expressions_part15] fragment 145 error: ${e.message}`);
    }}

    
// ---- fragment 146 ----
    try {{
        bCondition1 && bCondition2
    }} catch (e) {{
        console.error(`[test_expressions_part15] fragment 146 error: ${e.message}`);
    }}

    
// ---- fragment 147 ----
    try {{
        !(!bCondition1 || !bCondition2)
    }} catch (e) {{
        console.error(`[test_expressions_part15] fragment 147 error: ${e.message}`);
    }}

    
// ---- fragment 148 ----
    try {{
        bCondition1 || bCondition2
    }} catch (e) {{
        console.error(`[test_expressions_part15] fragment 148 error: ${e.message}`);
    }}

    
// ---- fragment 149 ----
    try {{
        !(!bCondition1 && !bCondition2)
    }} catch (e) {{
        console.error(`[test_expressions_part15] fragment 149 error: ${e.message}`);
    }}

    
}
module.exports = { test_expressions_part15 };
