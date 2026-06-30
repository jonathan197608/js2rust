// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 160-169)
// Generated: 2026-06-28

function test_builtins_part17() {
// ---- fragment 160 ----
    try {{
        const array = [1, 2, 3, 4, 5];
        for (const value of array) {
          if (value === 5) {
            continue;
          }
          // do something with value
        }
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 160 error: ${e.message}`);
    }}

    
// ---- fragment 161 ----
    try {{
        const obj = { a: 1, b: 2, c: 3 };

        for (const i in obj) {
          console.log(obj[i]);
        }
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 161 error: ${e.message}`);
    }}

    
// ---- fragment 162 ----
    try {{
        const arr = ["a", "b", "c"];

        for (let i = 2; i < arr.length; i++) {
          console.log(arr[i]);
        }

        // "c"
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 162 error: ${e.message}`);
    }}

    
// ---- fragment 163 ----
    try {{
        const life1 = "foo";
        const foo = life1;
        console.log(foo);
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 163 error: ${e.message}`);
    }}

    
// ---- fragment 164 ----
    try {{
        // Wrap the number in parentheses
        console.log(typeof (1).toString());

        // Add an extra dot for the number literal
        console.log(typeof (2).toString());

        // Use parentheses instead of square brackets for method access
        console.log(typeof (3).toString());
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 164 error: ${e.message}`);
    }}

    
// ---- fragment 165 ----
    try {{
        "This is actually a string";
        42 - 13;
        const foo = "bar";
        console.log(foo);
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 165 error: ${e.message}`);
    }}

    
// ---- fragment 166 ----
    try {{
        /1{1}/u;
        /1{1,}/u;
        /1{1,2}/u;
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 166 error: ${e.message}`);
    }}

    
// ---- fragment 167 ----
    try {{
        /[\(\)\{\}]/v;
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 167 error: ${e.message}`);
    }}

    
// ---- fragment 168 ----
    try {{
        // If you want to match NULL followed by a digit, use a character class
        /[\0]0/u;
        // If you want to match a character by its character value, use \x
        /\x01/u;
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 168 error: ${e.message}`);
    }}

    
// ---- fragment 169 ----
    try {{
        // There's no need to escape the space
        /[\f\v\n\t ]/u;
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 169 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part17 };
