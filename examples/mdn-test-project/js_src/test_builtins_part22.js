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
          return "Good effort";
        }

        console.log(cheer(147));
        console.log(cheer(120));
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 210 error: ${e.message}`);
    }}

    
// ---- fragment 211 ----
    try {{
        const a = 2;
        const b = 3;
        const r1 = (-a) ** b;
        const r2 = -(a ** b);
        console.log(r1);
        console.log(r2);
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 211 error: ${e.message}`);
    }}

    
// ---- fragment 212 ----
    try {{
        function taylorSin(x) {
          return x - (x * x * x) / 6;
        }

        console.log(taylorSin(1));
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 212 error: ${e.message}`);
    }}

    
// ---- fragment 213 ----
    try {{
        const warning1 = "Using //@ to indicate sourceURL pragmas is deprecated. Use //# instead";
        const warning2 = "Using //@ to indicate sourceMappingURL pragmas is deprecated. Use //# instead";
        console.log(warning1);
        console.log(warning2);
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 213 error: ${e.message}`);
    }}

    
// ---- fragment 214 ----
    try {{
        const obj1 = { key: 1 };
        console.log(obj1.key);
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 214 error: ${e.message}`);
    }}

    
// ---- fragment 215 ----
    try {{
        const obj2 = { key: "foo" };
        console.log(obj2.key);
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 215 error: ${e.message}`);
    }}

    
// ---- fragment 216 ----
    try {{
        const proto = {};
        console.log(proto);
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 216 error: ${e.message}`);
    }}

    
// ---- fragment 217 ----
    try {{
        const obj3 = {};
        console.log(obj3);
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 217 error: ${e.message}`);
    }}

    
// ---- fragment 218 ----
    try {{
        const circularReference = { otherData: 123 };
        circularReference.myself = circularReference;
        console.log(circularReference.otherData);
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 218 error: ${e.message}`);
    }}

    
// ---- fragment 219 ----
    try {{
        console.log("circular reference test");
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 219 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part22 };
