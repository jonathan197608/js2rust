// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 190-199)
// Generated: 2026-06-28

function test_builtins_part20() {
// ---- fragment 190 ----
    try {{
        const list = [1, 2];
        console.log(list);

        const instruments = ["Ukulele", "Guitar", "Piano"];
        console.log(instruments);

        const data = [{ foo: "bar" }];
        console.log(data);
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 190 error: ${e.message}`);
    }}

    
// ---- fragment 191 ----
    try {{
        const sunny = true;
        function charge() {
          if (sunny) {
            console.log("using solar cells");
          } else {
            console.log("prompt bike ride");
          }
        }
        charge();
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 191 error: ${e.message}`);
    }}

    
// ---- fragment 192 ----
    try {{
        const doSomething = function() { console.log("doing something"); };
        if (Math.random() < 0.01) {
            doSomething();
        }
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
        console.log(obj.a);
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 193 error: ${e.message}`);
    }}

    
// ---- fragment 194 ----
    try {{
        const COLUMNS = 80;
        console.log(COLUMNS);
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 194 error: ${e.message}`);
    }}

    
// ---- fragment 195 ----
    try {{
        let columns = 0;
        columns = 80;
        console.log(columns);
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

        console.log(square(2)); // 4
        console.log(greet("Howdy")); // "Howdy"
        log({ obj: "value" }); // { obj: "value" }
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 196 error: ${e.message}`);
    }}

    
// ---- fragment 197 ----
    try {{
        console.log("Hello" + "World");
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 197 error: ${e.message}`);
    }}

    
// ---- fragment 198 ----
    try {{
        const obj2 = { foo: { bar: "baz" } };
        console.log(obj2.foo.bar); // "baz"
        console.log(obj2["foo"]["bar"]); // "baz"
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
