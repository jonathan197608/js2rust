// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 190-199)
// Generated: 2026-06-28

function test_builtins_part20() {
// ---- fragment 190 ----
    try {{
        const list = [1, 2];

        const instruments = ["Ukulele", "Guitar", "Piano"];

        const data = [{ foo: "bar" }, { bar: "foo" }];
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 190 error: ${e.message}`);
    }}

    
// ---- fragment 191 ----
    try {{
        function charge() {
          if (sunny) {
            useSolarCells();
          } else {
            promptBikeRide();
          }
        }
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 191 error: ${e.message}`);
    }}

    
// ---- fragment 192 ----
    try {{
        (function () {
          if (Math.random() < 0.01) {
            doSomething();
          }
        })();
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 192 error: ${e.message}`);
    }}

    
// ---- fragment 193 ----
    try {{
        const obj = {
          a: 1,
          b: { myProp: 2 },
          c: 3,
        };
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 193 error: ${e.message}`);
    }}

    
// ---- fragment 194 ----
    try {{
        const COLUMNS = 80;
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 194 error: ${e.message}`);
    }}

    
// ---- fragment 195 ----
    try {{
        let columns;
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 195 error: ${e.message}`);
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
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 196 error: ${e.message}`);
    }}

    
// ---- fragment 197 ----
    try {{
        square(2); // 4

        greet("Howdy"); // "Howdy"

        log({ obj: "value" }); // { obj: "value" }
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 197 error: ${e.message}`);
    }}

    
// ---- fragment 198 ----
    try {{
        obj.foo.bar; // "baz"
        // or alternatively
        obj["foo"]["bar"]; // "baz"

        // computed properties require square brackets
        obj.foo["bar" + i]; // "baz2"
        // or as template literal
        obj.foo[`bar${i}`]; // "baz2"
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 198 error: ${e.message}`);
    }}

    
// ---- fragment 199 ----
    try {{
        console.log("Hello" + "World");
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 199 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part20 };
