// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 203-212)
// Generated: 2026-06-30

function test_builtins_part20() {
// ---- fragment 203 ----
try {{
        "\xA9";
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 203 error: ${e.message}`);
    }}

// ---- fragment 204 ----
try {{
        String.raw`\251`; // A string containing four characters
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 204 error: ${e.message}`);
    }}

// ---- fragment 205 ----
try {{
        function replacer(match, ...args) {
          const offset = args.at(-2);
          const string = args.at(-1);
        }

        function doSomething(arg1, arg2, ...otherArgs) {}
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 205 error: ${e.message}`);
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
        console.error(`[test_builtins_part20] fragment 206 error: ${e.message}`);
    }}

// ---- fragment 207 ----
try {{
        // All { and } need to be escaped
        /\{\{MDN_Macro\}\}/u;
        // The ] needs to be escaped
        /\[sic\]/u;
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 207 error: ${e.message}`);
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
        console.error(`[test_builtins_part20] fragment 208 error: ${e.message}`);
    }}

// ---- fragment 209 ----
try {{
        function doSomething(...args) {
          // args is always an array
        }
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 209 error: ${e.message}`);
    }}

// ---- fragment 210 ----
try {{
        function cheer(score) {
          if (score === 147) {
            return "Maximum!";
          }
          if (score > 100) {
            return "Century!";
          }
        }
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 210 error: ${e.message}`);
    }}

// ---- fragment 211 ----
try {{
        (-a) ** b
        -(a ** b)
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 211 error: ${e.message}`);
    }}

// ---- fragment 212 ----
try {{
        function taylorSin(x) {
          return (n) => ((-1) ** n * x ** (2 * n + 1)) / factorial(2 * n + 1);
        }
    }} catch (e) {{
        console.error(`[test_builtins_part20] fragment 212 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part20 };
