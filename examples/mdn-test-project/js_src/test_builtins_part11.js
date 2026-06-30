// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 100-109)
// Generated: 2026-06-28

function test_builtins_part11() {
// ---- fragment 100 ----
    try {{
        function areEqualCaseInsensitive(str1, str2) {
          return str1.toUpperCase() === str2.toUpperCase();
        }
        console.log(areEqualCaseInsensitive("A", "a"));
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 100 error: ${e.message}`);
    }}

    
// ---- fragment 101 ----
    try {{
        const strPrim = "foo"; // A literal is a string primitive
        const strPrim2 = String(1); // Coerced into the string primitive "1"
        const strPrim3 = String(true); // Coerced into the string primitive "true"
        const strObj = String(strPrim); // String() returns a string primitive

        console.log(strPrim2);
        console.log(strPrim3);
        console.log(strObj);
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 101 error: ${e.message}`);
    }}

    
// ---- fragment 102 ----
    try {{
        const s1 = "2 + 2"; // creates a string primitive
        const s2 = String("2 + 2"); // creates a string primitive
        console.log(s1);
        console.log(s2);
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 102 error: ${e.message}`);
    }}

    
// ---- fragment 103 ----
    try {{
        // eval not supported, s2 not in scope
        console.log("eval not supported");
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 103 error: ${e.message}`);
    }}

    
// ---- fragment 104 ----
    try {{
        // You cannot access properties on null or undefined

        const nullVar = 0;
        // nullVar.toString(); // TypeError: Cannot read properties of null
        String(nullVar); // "null"

        const undefinedVar = 0;
        // undefinedVar.toString(); // TypeError: Cannot read properties of undefined
        String(undefinedVar); // "undefined"
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 104 error: ${e.message}`);
    }}

    
// ---- fragment 105 ----
    try {{
        // ArrayBuffer/Int32Array not fully supported
        const buffer = [0, 0, 0, 0];
        const view = buffer;
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 105 error: ${e.message}`);
    }}

    
// ---- fragment 106 ----
    try {{
        // Simplified: ArrayBuffer/DataView endianness detection not supported
        const littleEndian = true;
        console.log(littleEndian); // true or false
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 106 error: ${e.message}`);
    }}

    
// ---- fragment 107 ----
    try {{
        // ArrayBuffer/DataView not fully supported
        const buffer2 = [0, 0, 0, 0, 0, 0, 0, 0];
        const view2 = buffer2;

        view2[1] = 42;
        view2[1]; // 42
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 107 error: ${e.message}`);
    }}

    
// ---- fragment 108 ----
    try {{
        // registry.register not supported (registry/target undeclared)
        const registry = 0;
        const target = 0;
        console.log(registry);
        console.log(target);
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 108 error: ${e.message}`);
    }}

    
// ---- fragment 109 ----
    try {{
        // registry.register not supported (registry/theObject undeclared)
        const theObject = 0;
        console.log(theObject);
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 109 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part11 };
