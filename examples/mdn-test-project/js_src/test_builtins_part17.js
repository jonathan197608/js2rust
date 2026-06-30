// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 173-182)
// Generated: 2026-06-30

function test_builtins_part17() {
// ---- fragment 173 ----
try {{
        /foo/g;
        /foo/gims;
        /foo/uy;
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 173 error: ${e.message}`);
    }}

// ---- fragment 174 ----
try {{
        const obj = {
          url: "/docs/Web",
        };
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 174 error: ${e.message}`);
    }}

// ---- fragment 175 ----
try {{
        /\u0065/u; // Lowercase "e"
        /\u{1f600}/u; // Grinning face emoji
        /\cA/u; // U+0001 (Start of Heading)
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 175 error: ${e.message}`);
    }}

// ---- fragment 176 ----
try {{
        JSON.parse("[1, 2, 3, 4,]");
        JSON.parse('{"foo": 1,}');
        // SyntaxError JSON.parse: unexpected character
        // at line 1 column 14 of the JSON data
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 176 error: ${e.message}`);
    }}

// ---- fragment 177 ----
try {{
        JSON.parse("[1, 2, 3, 4]");
        JSON.parse('{"foo": 1}');
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 177 error: ${e.message}`);
    }}

// ---- fragment 178 ----
try {{
        JSON.parse("{'foo': 1}");
        // SyntaxError: JSON.parse: expected property name or '}'
        // at line 1 column 2 of the JSON data
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 178 error: ${e.message}`);
    }}

// ---- fragment 179 ----
try {{
        JSON.parse('{"foo": 1}');
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 179 error: ${e.message}`);
    }}

// ---- fragment 180 ----
try {{
        JSON.parse('{"foo": 01}');
        // SyntaxError: JSON.parse: expected ',' or '}' after property value
        // in object at line 1 column 2 of the JSON data

        JSON.parse('{"foo": 1.}');
        // SyntaxError: JSON.parse: unterminated fractional number
        // at line 1 column 2 of the JSON data
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 180 error: ${e.message}`);
    }}

// ---- fragment 181 ----
try {{
        JSON.parse('{"foo": 1}');
        JSON.parse('{"foo": 1.0}');
    }} catch (e) {{
        console.error(`[test_builtins_part17] fragment 181 error: ${e.message}`);
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
        console.error(`[test_builtins_part17] fragment 182 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part17 };
