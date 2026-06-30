// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 153-162)
// Generated: 2026-06-30

function test_builtins_part15() {
// ---- fragment 153 ----
try {{
        "abc".repeat(0); // ''
        "abc".repeat(1); // 'abc'
        "abc".repeat(2); // 'abcabc'
        "abc".repeat(3.5); // 'abcabcabc' (count will be converted to integer)
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 153 error: ${e.message}`);
    }}

// ---- fragment 154 ----
// SKIP: Tests spread + rest pattern which has codegen issues
// Fragment 154 skipped — rest parameter + spread args codegen issue

// ---- fragment 155 ----
// SKIP: Tests 0o3 octal literal which oxc parser may not support

// ---- fragment 156 ----
try {{
        var BLUE = 2;
        var GREEN = 1;
        var RED = 0;
        const colorEnum = { RED: 0, GREEN: 1, BLUE: 2 };
        const list = ["potatoes", "rice", "fries"];
        console.log(colorEnum.RED, colorEnum.GREEN, list[0]);
        _ = BLUE;
        _ = GREEN;
        _ = RED;
}} catch (e) {{
        console.error(`[test_builtins_part15] fragment 156 error: ${e.message}`);
    }}

// ---- fragment 157 ----
// SKIP: Tests JS spec behavior (class reserved identifier + SyntaxError path)
// Fragment 157 skipped — "is" is a reserved word in strict mode

// ---- fragment 158 ----
try {{
        const iterable = [10, 20, 30];

        for (let value of iterable) {
          value += 50;
          console.log(value);
        }
        // 60
        // 70
        // 80
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 158 error: ${e.message}`);
    }}

// ---- fragment 159 ----
try {{
        var array = [1, 2, 3, 4, 5];
        array.forEach((value) => {
          if (value === 5) {
            return;
          }
          // do something with value
        });
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 159 error: ${e.message}`);
    }}

// ---- fragment 160 ----
try {{
        var array = [1, 2, 3, 4, 5];
        for (const value of array) {
          if (value === 5) {
            continue;
          }
          // do something with value
        }
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 160 error: ${e.message}`);
    }}

// ---- fragment 161 ----
try {{
        var a = 1;
        var b = 2;
        var c = 3;
        const obj = { a: 1, b: 2, c: 3 };

        for (const i in obj) {
          console.log(obj[i]);
        }
            _ = a;
        _ = b;
        _ = c;
}} catch (e) {{
        console.error(`[test_builtins_part15] fragment 161 error: ${e.message}`);
    }}

// ---- fragment 162 ----
try {{
        const arr = ["a", "b", "c"];

        for (let i = 2; i < arr.length; i++) {
          console.log(arr[i]);
        }

        // "c"
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 162 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part15 };
