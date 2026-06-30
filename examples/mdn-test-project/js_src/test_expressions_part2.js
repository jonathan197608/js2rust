// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 10-19)
// Generated: 2026-06-30

function test_expressions_part2() {
// ---- fragment 10 ----
try {{
        ~0; // -1
        ~-1; // 0
        ~1; // -2

        ~0n; // -1n
        ~4294967295n; // -4294967296n
    }} catch (e) {{
        console.error(`[test_expressions_part2] fragment 10 error: ${e.message}`);
    }}

// ---- fragment 11 ----
try {{
        const a = 3;
        const b = -2;

        console.log(!(a > 0 || b > 0));
    }} catch (e) {{
        console.error(`[test_expressions_part2] fragment 11 error: ${e.message}`);
    }}

// ---- fragment 12 ----
try {{
        var x = 1;
        !x
    }} catch (e) {{
        console.error(`[test_expressions_part2] fragment 12 error: ${e.message}`);
    }}

// ---- fragment 13 ----
try {{
        !true; // !t returns false
        !false; // !f returns true
        !""; // !f returns true
        !"Cat"; // !t returns false
    }} catch (e) {{
        console.error(`[test_expressions_part2] fragment 13 error: ${e.message}`);
    }}

// ---- fragment 14 ----
try {{
        var bCondition = 0;
        !!bCondition
    }} catch (e) {{
        console.error(`[test_expressions_part2] fragment 14 error: ${e.message}`);
    }}

// ---- fragment 15 ----
try {{
        var bCondition = 0;
        bCondition
    }} catch (e) {{
        console.error(`[test_expressions_part2] fragment 15 error: ${e.message}`);
    }}

// ---- fragment 16 ----
try {{
        console.log(3 ** 4);

        console.log(10 ** -2);

        console.log(2 ** (3 ** 2));

        console.log((2 ** 3) ** 2);
    }} catch (e) {{
        console.error(`[test_expressions_part2] fragment 16 error: ${e.message}`);
    }}

// ---- fragment 17 ----
try {{
        var x = 1;
        var y = 2;
        x ** y
    }} catch (e) {{
        console.error(`[test_expressions_part2] fragment 17 error: ${e.message}`);
    }}

// ---- fragment 18 ----
try {{
        2 ** 3; // 8
        3 ** 2; // 9
        3 ** 2.5; // 15.588457268119896
        10 ** -1; // 0.1
        2 ** 1024; // Infinity
        NaN ** 2; // NaN
        NaN ** 0; // 1
        1 ** Infinity; // NaN
    }} catch (e) {{
        console.error(`[test_expressions_part2] fragment 18 error: ${e.message}`);
    }}

// ---- fragment 19 ----
try {{
        2 ** "3"; // 8
        2 ** "hello"; // NaN
    }} catch (e) {{
        console.error(`[test_expressions_part2] fragment 19 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part2 };
