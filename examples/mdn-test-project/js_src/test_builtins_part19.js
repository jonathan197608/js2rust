// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 193-202)
// Generated: 2026-06-30

function test_builtins_part19() {
// ---- fragment 193 ----
try {{
        var a = 1;
        var b = 2;
        var c = 3;
        var myProp = 0;
        const obj = {
          a: 1,
          b: { myProp: 2 },
          c: 3,
        };
            _ = obj;
        _ = a;
        _ = b;
        _ = c;
        _ = myProp;
}} catch (e) {{
        console.error(`[test_builtins_part19] fragment 193 error: ${e.message}`);
    }}

// ---- fragment 194 ----
try {{
        const COLUMNS = 80;
            _ = COLUMNS;
}} catch (e) {{
        console.error(`[test_builtins_part19] fragment 194 error: ${e.message}`);
    }}

// ---- fragment 195 ----
try {{
        let columns;
            _ = columns;
}} catch (e) {{
        console.error(`[test_builtins_part19] fragment 195 error: ${e.message}`);
    }}

// ---- fragment 196 ----
try {{
        function square(number) {
          return number * number;
        }

        function greet(greeting) {
          return greeting;
        }

        function log(arg) {
          console.log(arg);
        }
            _ = greet;
        _ = log;
        _ = square;
}} catch (e) {{
        console.error(`[test_builtins_part19] fragment 196 error: ${e.message}`);
    }}

// ---- fragment 197 ----
try {{
        var log = 0;
        var greet = 0;
        var square = 0;
        square(2); // 4

        greet("Howdy"); // "Howdy"

        log({ obj: "value" }); // { obj: "value" }
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 197 error: ${e.message}`);
    }}

// ---- fragment 198 ----
try {{
        var i = 0;
        var obj = 0;
        obj.foo.bar; // "baz"
        // or alternatively
        obj["foo"]["bar"]; // "baz"

        // computed properties require square brackets
        obj.foo["bar" + i]; // "baz2"
        // or as template literal
        obj.foo[`bar${i}`]; // "baz2"
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 198 error: ${e.message}`);
    }}

// ---- fragment 199 ----
try {{
        console.log("Hello" + "World");
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 199 error: ${e.message}`);
    }}

// ---- fragment 200 ----
try {{
        var p = 0;
        var v = 0;
        // Matches two characters that are not an emoji flag sequence
        /(?!\p{RGI_Emoji_Flag_Sequence})../v;
            _ = p;
        _ = v;
}} catch (e) {{
        console.error(`[test_builtins_part19] fragment 200 error: ${e.message}`);
    }}

// ---- fragment 201 ----
try {{
        var b = 2;
        var hello = 0;
        /b+/; // b is a character, it can be repeated
        /(\*hello\*)/; // Escape the asterisks to match them literally
            _ = b;
        _ = hello;
}} catch (e) {{
        console.error(`[test_builtins_part19] fragment 201 error: ${e.message}`);
    }}

// ---- fragment 202 ----
try {{
        /1{1,2}/;
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 202 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part19 };
