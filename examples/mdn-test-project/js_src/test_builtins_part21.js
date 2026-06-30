// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 200-209)
// Generated: 2026-06-28
// Note: RegExp literals simplified to string literals

function test_builtins_part21() {
// ---- fragment 200 ----
    try {{
        const regexStr = "emoji flag sequence";
        console.log(regexStr);
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 200 error: ${e.message}`);
    }}

    
// ---- fragment 201 ----
    try {{
        const r1 = "b+";
        const r2 = "*hello*";
        console.log(r1);
        console.log(r2);
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 201 error: ${e.message}`);
    }}

    
// ---- fragment 202 ----
    try {{
        const r3 = "1{1,2}";
        console.log(r3);
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 202 error: ${e.message}`);
    }}

    
// ---- fragment 203 ----
    try {{
        const copyright = "\xA9";
        console.log(copyright);
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 203 error: ${e.message}`);
    }}

    
// ---- fragment 204 ----
    try {{
        const raw = "\\251";
        console.log(raw);
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 204 error: ${e.message}`);
    }}

    
// ---- fragment 205 ----
    try {{
        function replacer(match) {
          const offset = 0;
          console.log(match);
          return offset;
        }

        console.log(replacer("test"));
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 205 error: ${e.message}`);
    }}

    
// ---- fragment 206 ----
    try {{
        const obj = { __proto__: { a: 1 } };
        console.log(obj.a);

        const __proto__ = 0;
        console.log(__proto__);

        const obj2 = {
          ["__proto__"]: {},
        };
        console.log(obj2);
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 206 error: ${e.message}`);
    }}

    
// ---- fragment 207 ----
    try {{
        const macro = "MDN_Macro";
        const sic = "sic";
        console.log(macro);
        console.log(sic);
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 207 error: ${e.message}`);
    }}

    
// ---- fragment 208 ----
    try {{
        function f(arg) {
          arg = "foo";
          return arg;
        }

        function g(arg) {
          let bar = "foo";
          return bar;
        }

        console.log(f("test"));
        console.log(g("test"));
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 208 error: ${e.message}`);
    }}

    
// ---- fragment 209 ----
    try {{
        function doSomething(...args) {
          console.log(args.length);
        }

        doSomething(1, 2, 3);
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 209 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part21 };
