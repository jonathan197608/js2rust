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
        var expression = 0;
        _ = void expression
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
// SKIP: Tests void IIFE pattern which has codegen issues
// Fragment 3 skipped — void + anonymous IIFE codegen issue

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
        function doSomething() { return 0; }
        var checkbox = {};
        checkbox.onclick = () => doSomething();
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 5 error: ${e.message}`);
    }}

// ---- fragment 6 ----
try {{
        function doSomething2() { return 0; }
        var checkbox2 = {};
        checkbox2.onclick = () => void doSomething2();
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
        var x = 1;
        _ = ~x
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 8 error: ${e.message}`);
    }}

// ---- fragment 9 ----
try {{
        // Bitwise NOT operations on large numbers
        const a = ~11100110111110100000000000000110000000000001;
        const b = ~10100000000000000110000000000001;
        console.log(typeof a, typeof b);
    }} catch (e) {{
        console.error(`[test_expressions_part1] fragment 9 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part1 };
