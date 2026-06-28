// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 170-179)
// Generated: 2026-06-28

function test_builtins_part18() {
// ---- fragment 170 ----
    try {{
        /\p{Script=Latin}/u; // "Script=Latin" is a valid Unicode property
        /\p{Letter}/u; // "Letter" is valid value for General_Category
        /\p{RGI_Emoji_Flag_Sequence}/v; // Property of strings can only be used in "v" mode
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 170 error: ${e.message}`);
    }}

    
// ---- fragment 171 ----
    try {{
        /[1-9]/; // Swap the range
        /[_\-=]/; // Escape the hyphen so it matches the literal character
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 171 error: ${e.message}`);
    }}

    
// ---- fragment 172 ----
    try {{
        const re = new RegExp("pattern", "flags");
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 172 error: ${e.message}`);
    }}

    
// ---- fragment 173 ----
    try {{
        /foo/g;
        /foo/gims;
        /foo/uy;
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 173 error: ${e.message}`);
    }}

    
// ---- fragment 174 ----
    try {{
        const obj = {
          url: "/docs/Web",
        };
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 174 error: ${e.message}`);
    }}

    
// ---- fragment 175 ----
    try {{
        /\u0065/u; // Lowercase "e"
        /\u{1f600}/u; // Grinning face emoji
        /\cA/u; // U+0001 (Start of Heading)
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 175 error: ${e.message}`);
    }}

    
// ---- fragment 176 ----
    try {{
        JSON.parse("[1, 2, 3, 4,]");
        JSON.parse('{"foo": 1,}');
        // SyntaxError JSON.parse: unexpected character
        // at line 1 column 14 of the JSON data
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 176 error: ${e.message}`);
    }}

    
// ---- fragment 177 ----
    try {{
        JSON.parse("[1, 2, 3, 4]");
        JSON.parse('{"foo": 1}');
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 177 error: ${e.message}`);
    }}

    
// ---- fragment 178 ----
    try {{
        JSON.parse("{'foo': 1}");
        // SyntaxError: JSON.parse: expected property name or '}'
        // at line 1 column 2 of the JSON data
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 178 error: ${e.message}`);
    }}

    
// ---- fragment 179 ----
    try {{
        JSON.parse('{"foo": 1}');
    }} catch (e) {{
        console.error(`[test_builtins_part18] fragment 179 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part18 };
