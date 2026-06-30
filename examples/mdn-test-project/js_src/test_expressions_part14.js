// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 130-139)
// Generated: 2026-06-30

function test_expressions_part14() {
// ---- fragment 130 ----
try {{
        var x = 1;
        var y = 2;
        x && y
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 130 error: ${e.message}`);
    }}

// ---- fragment 131 ----
try {{
        var result = 0;
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
        var a6 = false;
        var a7 = false;
        var a1 = false;
        var a2 = false;
        var a3 = false;
        var a4 = false;
        var a5 = false;
        var a8 = false;
        var a9 = false;
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
        var bCondition1 = true;
        var bCondition2 = false;
        bCondition1 && bCondition2
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 135 error: ${e.message}`);
    }}

// ---- fragment 136 ----
try {{
        var bCondition1 = true;
        var bCondition2 = false;
        !(!bCondition1 || !bCondition2)
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 136 error: ${e.message}`);
    }}

// ---- fragment 137 ----
try {{
        var bCondition1 = true;
        var bCondition2 = false;
        bCondition1 || bCondition2
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 137 error: ${e.message}`);
    }}

// ---- fragment 138 ----
try {{
        var bCondition1 = true;
        var bCondition2 = false;
        !(!bCondition1 && !bCondition2)
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 138 error: ${e.message}`);
    }}

// ---- fragment 139 ----
try {{
        var bCondition1 = true;
        var bCondition2 = false;
        var bCondition3 = true;
        bCondition1 || (bCondition2 && bCondition3)
    }} catch (e) {{
        console.error(`[test_expressions_part14] fragment 139 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part14 };
