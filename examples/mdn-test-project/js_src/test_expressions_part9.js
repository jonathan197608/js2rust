// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 80-89)
// Generated: 2026-06-30

function test_expressions_part9() {
// ---- fragment 80 ----
try {{
        "5" >= 3; // true
        "3" >= 3; // true
        "3" >= 5; // false

        "hello" >= 5; // false
        5 >= "hello"; // false
    }} catch (e) {{
        console.error(`[test_expressions_part9] fragment 80 error: ${e.message}`);
    }}

// ---- fragment 81 ----
try {{
        5 >= 3; // true
        3 >= 3; // true
        3 >= 5; // false
    }} catch (e) {{
        console.error(`[test_expressions_part9] fragment 81 error: ${e.message}`);
    }}

// ---- fragment 82 ----
try {{
        5n >= 3; // true
        3 >= 3n; // true
        3 >= 5n; // false
    }} catch (e) {{
        console.error(`[test_expressions_part9] fragment 82 error: ${e.message}`);
    }}

// ---- fragment 83 ----
try {{
        true >= false; // true
        true >= true; // true
        false >= true; // false

        true >= 0; // true
        true >= 1; // true

        null >= 0; // true
        1 >= null; // true

        undefined >= 3; // false
        3 >= undefined; // false

        3 >= NaN; // false
        NaN >= 3; // false
    }} catch (e) {{
        console.error(`[test_expressions_part9] fragment 83 error: ${e.message}`);
    }}

// ---- fragment 84 ----
try {{
        console.log(1 != 1);

        console.log("hello" != "hello");

        console.log("1" != 1);

        console.log(0 != false);
    }} catch (e) {{
        console.error(`[test_expressions_part9] fragment 84 error: ${e.message}`);
    }}

// ---- fragment 85 ----
try {{
        var x = 1;
        var y = 2;
        x != y
    }} catch (e) {{
        console.error(`[test_expressions_part9] fragment 85 error: ${e.message}`);
    }}

// ---- fragment 86 ----
try {{
        var x = 1;
        var y = 2;
        x != y;

        !(x == y);
    }} catch (e) {{
        console.error(`[test_expressions_part9] fragment 86 error: ${e.message}`);
    }}

// ---- fragment 87 ----
try {{
        3 != "3"; // false
    }} catch (e) {{
        console.error(`[test_expressions_part9] fragment 87 error: ${e.message}`);
    }}

// ---- fragment 88 ----
try {{
        3 !== "3"; // true
    }} catch (e) {{
        console.error(`[test_expressions_part9] fragment 88 error: ${e.message}`);
    }}

// ---- fragment 89 ----
try {{
        1 != 2; // true
        "hello" != "hola"; // true

        1 != 1; // false
        "hello" != "hello"; // false
    }} catch (e) {{
        console.error(`[test_expressions_part9] fragment 89 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part9 };
