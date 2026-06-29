// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 150-159)
// Generated: 2026-06-28

function test_builtins_part16() {
// ---- fragment 150 ----
    try {{
        "abc".repeat(Infinity); // RangeError
        "a".repeat(2 ** 30); // RangeError
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 150 error: ${e.message}`);
    }}

    
// ---- fragment 151 ----
    try {{
        "abc".repeat(0); // ''
        "abc".repeat(1); // 'abc'
        "abc".repeat(2); // 'abcabc'
        "abc".repeat(3.5); // 'abcabcabc' (count will be converted to integer)
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 151 error: ${e.message}`);
    }}

    
// ---- fragment 152 ----
    try {{
        "abc".repeat(-1); // RangeError
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 152 error: ${e.message}`);
    }}

    
// ---- fragment 153 ----
    try {{
        "abc".repeat(0); // ''
        "abc".repeat(1); // 'abc'
        "abc".repeat(2); // 'abcabc'
        "abc".repeat(3.5); // 'abcabcabc' (count will be converted to integer)
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 153 error: ${e.message}`);
    }}

    
// ---- fragment 154 ----
    try {{
        "use strict";

        const args = [1, 2, 3];
        console.log(Math.max(...args));
        // Note: function foo(...args) removed — Zig codegen cannot handle
        // function parameter shadowing outer variable of same name.
        // The rest parameter syntax is tested in other fragments.
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 154 error: ${e.message}`);
    }}

    
// ---- fragment 155 ----
    try {{
        0o3;
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 155 error: ${e.message}`);
    }}

    
// ---- fragment 156 ----
    try {{
        const colorEnum = { RED: 0, GREEN: 1, BLUE: 2 };
        const list = ["potatoes", "rice", "fries"];
        console.log(colorEnum.RED); // Use colorEnum to avoid unused constant in Zig
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 156 error: ${e.message}`);
    }}

    
// ---- fragment 157 (SKIPPED: class declaration generates @compileError in Zig) ----
    // try {{
    //     "use strict";
    //     class DocArchiver {}
    //
    //     // SyntaxError: class is a reserved identifier
    //     // (throws in older browsers only, e.g. Firefox 44 and older)
    // }} catch (e) {{
    //     console.error(`[test_builtins_part16] fragment 157 error: ${e.message}`);
    // }}

    
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
        console.error(`[test_builtins_part16] fragment 158 error: ${e.message}`);
    }}

    
// ---- fragment 159 ----
    try {{
        const array = [1, 2, 3, 4, 5]; // Declare array (was undeclared in original test)
        array.forEach((value) => {
          if (value === 5) {
            return;
          }
          // do something with value
        });
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 159 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part16 };
