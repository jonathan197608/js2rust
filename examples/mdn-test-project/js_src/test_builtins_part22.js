// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 210-219)
// Generated: 2026-06-28

function test_builtins_part22() {
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
        console.error(`[test_builtins_part22] fragment 210 error: ${e.message}`);
    }}

    
// ---- fragment 211 ----
    try {{
        (-a) ** b
        -(a ** b)
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 211 error: ${e.message}`);
    }}

    
// ---- fragment 212 ----
    try {{
        function taylorSin(x) {
          return (n) => ((-1) ** n * x ** (2 * n + 1)) / factorial(2 * n + 1);
        }
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 212 error: ${e.message}`);
    }}

    
// ---- fragment 213 ----
    try {{
        Warning: SyntaxError: Using //@ to indicate sourceURL pragmas is deprecated. Use //# instead

        Warning: SyntaxError: Using //@ to indicate sourceMappingURL pragmas is deprecated. Use //# instead
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 213 error: ${e.message}`);
    }}

    
// ---- fragment 214 ----
    try {{
        Object.defineProperty({}, "key", 1);
        // TypeError: 1 is not a non-null object

        Object.defineProperty({}, "key", null);
        // TypeError: null is not a non-null object
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 214 error: ${e.message}`);
    }}

    
// ---- fragment 215 ----
    try {{
        Object.defineProperty({}, "key", { value: "foo", writable: false });
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 215 error: ${e.message}`);
    }}

    
// ---- fragment 216 ----
    try {{
        Object.setPrototypeOf(Object.prototype, {});
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 216 error: ${e.message}`);
    }}

    
// ---- fragment 217 ----
    try {{
        const obj = {};
        Object.preventExtensions(obj);
        Object.setPrototypeOf(obj, {});
        // TypeError: can't set prototype of this object
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 217 error: ${e.message}`);
    }}

    
// ---- fragment 218 ----
    try {{
        const circularReference = { otherData: 123 };
        circularReference.myself = circularReference;
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 218 error: ${e.message}`);
    }}

    
// ---- fragment 219 ----
    try {{
        JSON.stringify(circularReference);
        // TypeError: cyclic object value
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 219 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part22 };
