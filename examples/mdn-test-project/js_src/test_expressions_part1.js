// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 0-9)
// Generated: 2026-06-30

function test_expressions_part1() {
// ---- fragment 0 ----
try {{
        const output = void 1;
        console.log(output);

        void console.log("expression evaluated");

        void (function iife() {
          console.log("iife is executed");
        })();

        void function test() {
          console.log("test function executed");
        };
        try {
          test();
        } catch (e) {
          console.log("test function is not defined");
        }
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 0 error: ${e.message}`);
    }}

// ---- fragment 1 ----
try {{
        void expression
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 1 error: ${e.message}`);
    }}

// ---- fragment 2 ----
try {{
        void 2 === "2"; // (void 2) === '2', returns false
        void (2 === "2"); // void (2 === '2'), returns undefined
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 2 error: ${e.message}`);
    }}

// ---- fragment 3 ----
try {{
        void function () {
          console.log("Executed!");
        }();

        // Logs "Executed!"
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 3 error: ${e.message}`);
    }}

// ---- fragment 4 ----
try {{
        (function () {
          console.log("Executed!");
        })();
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 4 error: ${e.message}`);
    }}

// ---- fragment 5 ----
try {{
        checkbox.onclick = () => doSomething();
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 5 error: ${e.message}`);
    }}

// ---- fragment 6 ----
try {{
        checkbox.onclick = () => void doSomething();
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 6 error: ${e.message}`);
    }}

// ---- fragment 7 ----
try {{
        const a = 5; // 00000000000000000000000000000101
        const b = -3; // 11111111111111111111111111111101

        console.log(~a); // 11111111111111111111111111111010

        console.log(~b); // 00000000000000000000000000000010
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 7 error: ${e.message}`);
    }}

// ---- fragment 8 ----
try {{
        ~x
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 8 error: ${e.message}`);
    }}

// ---- fragment 9 ----
try {{
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 9 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part1 };
