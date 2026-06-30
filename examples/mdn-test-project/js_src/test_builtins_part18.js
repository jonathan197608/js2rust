// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 183-192)
// Generated: 2026-06-30

function test_builtins_part18() {
// ---- fragment 183 ----
try {{
        console.log("PI: " + Math.PI);
        // "PI: 3.141592653589793"
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 183 error: ${e.message}`);
    }}

// ---- fragment 184 ----
try {{
        console.log(`PI: ${Math.PI}`);
        console.log("PI:", Math.PI);
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 184 error: ${e.message}`);
    }}

// ---- fragment 185 ----
try {{
        console.log('"Java" + "Script" = "' + "Java" + 'Script"');
        // '"Java" + "Script" = "JavaScript"'
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 185 error: ${e.message}`);
    }}

// ---- fragment 186 ----
try {{
        var condition = 0;
        if (condition) {
          // do something if the condition is true
        }
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 186 error: ${e.message}`);
    }}

// ---- fragment 187 ----
try {{
        if (Math.PI < 3) {
          console.log("wait what?");
        }
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 187 error: ${e.message}`);
    }}

// ---- fragment 188 ----
try {{
        var done = false;
        if (done === true) {
          console.log("we are done!");
        }
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 188 error: ${e.message}`);
    }}

// ---- fragment 189 ----
try {{
        var done = false;
        if (done) {
          console.log("we are done!");
        }
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 189 error: ${e.message}`);
    }}

// ---- fragment 190 ----
try {{
        var foo = 0;
        const list = [1, 2];

        const instruments = ["Ukulele", "Guitar", "Piano"];

        const data = [{ foo: "bar" }, { bar: "foo" }];
            _ = data;
        _ = instruments;
        _ = list;
        _ = foo;
}} catch (e) {{
        console.error(`[test_builtins_part18] fragment 190 error: ${e.message}`);
    }}

// ---- fragment 191 ----
try {{
        var promptBikeRide = 0;
        var sunny = 0;
        var useSolarCells = 0;
        function charge() {
          if (sunny) {
            useSolarCells();
          } else {
            promptBikeRide();
          }
        }
            _ = charge;
}} catch (e) {{
        console.error(`[test_builtins_part18] fragment 191 error: ${e.message}`);
    }}

// ---- fragment 192 ----
try {{
        function doSomething() { return 0; }
        (function () {
          if (Math.random() < 0.01) {
            doSomething();
          }
        })();
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 192 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part18 };
