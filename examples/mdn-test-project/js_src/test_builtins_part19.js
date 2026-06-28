// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 180-189)
// Generated: 2026-06-28

function test_builtins_part19() {
// ---- fragment 180 ----
    try {{
        JSON.parse('{"foo": 01}');
        // SyntaxError: JSON.parse: expected ',' or '}' after property value
        // in object at line 1 column 2 of the JSON data

        JSON.parse('{"foo": 1.}');
        // SyntaxError: JSON.parse: unterminated fractional number
        // at line 1 column 2 of the JSON data
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 180 error: ${e.message}`);
    }}

    
// ---- fragment 181 ----
    try {{
        JSON.parse('{"foo": 1}');
        JSON.parse('{"foo": 1.0}');
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 181 error: ${e.message}`);
    }}

    
// ---- fragment 182 ----
    try {{
        start: {
          console.log("Hello, world!");
          if (Math.random() > 0.5) {
            break start;
          }
          console.log("Maybe I'm logged");
        }
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 182 error: ${e.message}`);
    }}

    
// ---- fragment 183 ----
    try {{
        console.log("PI: " + Math.PI);
        // "PI: 3.141592653589793"
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 183 error: ${e.message}`);
    }}

    
// ---- fragment 184 ----
    try {{
        console.log(`PI: ${Math.PI}`);
        console.log("PI:", Math.PI);
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 184 error: ${e.message}`);
    }}

    
// ---- fragment 185 ----
    try {{
        console.log('"Java" + "Script" = "' + "Java" + 'Script"');
        // '"Java" + "Script" = "JavaScript"'
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 185 error: ${e.message}`);
    }}

    
// ---- fragment 186 ----
    try {{
        if (condition) {
          // do something if the condition is true
        }
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 186 error: ${e.message}`);
    }}

    
// ---- fragment 187 ----
    try {{
        if (Math.PI < 3) {
          console.log("wait what?");
        }
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 187 error: ${e.message}`);
    }}

    
// ---- fragment 188 ----
    try {{
        if (done === true) {
          console.log("we are done!");
        }
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 188 error: ${e.message}`);
    }}

    
// ---- fragment 189 ----
    try {{
        if (done) {
          console.log("we are done!");
        }
    }} catch (e) {{
        console.error(`[test_builtins_part19] fragment 189 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part19 };
