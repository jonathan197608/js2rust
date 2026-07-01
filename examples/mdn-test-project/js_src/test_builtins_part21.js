// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 200-209)
// Generated: 2026-06-28

function test_builtins_part21() {
// ---- fragment 200 ----
    try {{
        // Matches two characters that are not an emoji flag sequence
        /(?!\p{RGI_Emoji_Flag_Sequence})../v;
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 200 error: ${e.message}`);
    }}

    
// ---- fragment 201 ----
    try {{
        /b+/; // b is a character, it can be repeated
        /(\*hello\*)/; // Escape the asterisks to match them literally
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 201 error: ${e.message}`);
    }}

    
// ---- fragment 202 ----
    try {{
        /1{1,2}/;
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 202 error: ${e.message}`);
    }}

    
// ---- fragment 203 ----
    try {{
        "\xA9";
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 203 error: ${e.message}`);
    }}

    
// ---- fragment 204 ----
    try {{
        String.raw`\251`; // A string containing four characters
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 204 error: ${e.message}`);
    }}

    
// ---- fragment 205 ----
    try {{
        function replacer(match, ...args) {
          const offset = args.at(-2);
          const string = args.at(-1);
        }

        function doSomething(arg1, arg2, ...otherArgs) {}
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 205 error: ${e.message}`);
    }}

    
// ---- fragment 206 ----
    try {{
        // Only setting the prototype once
        const obj = { __proto__: { a: 1 } };

        // These syntaxes all create a property called "__proto__" and can coexist
        // They would overwrite each other and the last one is actually used
        const __proto__ = null;
        const obj2 = {
          ["__proto__"]: {},
          __proto__,
          __proto__() {},
          get __proto__() {
            return 1;
          },
        };
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 206 error: ${e.message}`);
    }}

    
// ---- fragment 207 ----
    try {{
        // All { and } need to be escaped
        /\{\{MDN_Macro\}\}/u;
        // The ] needs to be escaped
        /\[sic\]/u;
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 207 error: ${e.message}`);
    }}

    
// ---- fragment 208 ----
    try {{
        function f(arg) {
          arg = "foo";
        }

        function g(arg) {
          let bar = "foo";
        }
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 208 error: ${e.message}`);
    }}

    
// ---- fragment 209 ----
    try {{
        function doSomething(...args) {
          // args is always an array
        }
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 209 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part21 };
