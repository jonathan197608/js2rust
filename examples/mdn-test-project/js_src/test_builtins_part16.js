// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 163-172)
// Generated: 2026-06-30

function test_builtins_part16() {
// ---- fragment 163 ----
try {{
        const life1 = "foo";
        const foo = life1;
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 163 error: ${e.message}`);
    }}

// ---- fragment 164 ----
try {{
        // Wrap the number in parentheses
        alert(typeof (1).toString());

        // Add an extra dot for the number literal
        alert(typeof 2..toString());

        // Use square brackets
        alert(typeof 3["toString"]());
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 164 error: ${e.message}`);
    }}

// ---- fragment 165 ----
try {{
        "This is actually a string";
        42 - 13;
        const foo = "bar";
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 165 error: ${e.message}`);
    }}

// ---- fragment 166 ----
try {{
        /1{1}/u;
        /1{1,}/u;
        /1{1,2}/u;
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 166 error: ${e.message}`);
    }}

// ---- fragment 167 ----
try {{
        /[\(\)\{\}]/v;
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 167 error: ${e.message}`);
    }}

// ---- fragment 168 ----
try {{
        // If you want to match NULL followed by a digit, use a character class
        /[\0]0/u;
        // If you want to match a character by its character value, use \x
        /\x01/u;
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 168 error: ${e.message}`);
    }}

// ---- fragment 169 ----
try {{
        // There's no need to escape the space
        /[\f\v\n\t ]/u;
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 169 error: ${e.message}`);
    }}

// ---- fragment 170 ----
try {{
        /\p{Script=Latin}/u; // "Script=Latin" is a valid Unicode property
        /\p{Letter}/u; // "Letter" is valid value for General_Category
        /\p{RGI_Emoji_Flag_Sequence}/v; // Property of strings can only be used in "v" mode
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 170 error: ${e.message}`);
    }}

// ---- fragment 171 ----
try {{
        /[1-9]/; // Swap the range
        /[_\-=]/; // Escape the hyphen so it matches the literal character
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 171 error: ${e.message}`);
    }}

// ---- fragment 172 ----
try {{
        const re = new RegExp("pattern", "flags");
    }} catch (e) {{
        console.error(`[test_builtins_part16] fragment 172 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part16 };
