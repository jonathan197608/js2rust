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
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 100 error: ${e.message}`);
    }}

    
// ---- fragment 101 ----
    try {{
        const strPrim = "foo"; // A literal is a string primitive
        const strPrim2 = String(1); // Coerced into the string primitive "1"
        const strPrim3 = String(true); // Coerced into the string primitive "true"
        const strObj = new String(strPrim); // String with new returns a string wrapper object.

        console.log(typeof strPrim); // "string"
        console.log(typeof strPrim2); // "string"
        console.log(typeof strPrim3); // "string"
        console.log(typeof strObj); // "object"
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 101 error: ${e.message}`);
    }}

    
// ---- fragment 102 ----
    try {{
        const s1 = "2 + 2"; // creates a string primitive
        const s2 = new String("2 + 2"); // creates a String object
        console.log(eval(s1)); // returns the number 4
        console.log(eval(s2)); // returns the string "2 + 2"
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 102 error: ${e.message}`);
    }}

    
// ---- fragment 103 ----
    try {{
        console.log(eval(s2.valueOf())); // returns the number 4
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 103 error: ${e.message}`);
    }}

    
// ---- fragment 104 ----
    try {{
        // You cannot access properties on null or undefined

        const nullVar = null;
        nullVar.toString(); // TypeError: Cannot read properties of null
        String(nullVar); // "null"

        const undefinedVar = undefined;
        undefinedVar.toString(); // TypeError: Cannot read properties of undefined
        String(undefinedVar); // "undefined"
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 104 error: ${e.message}`);
    }}

    
// ---- fragment 105 ----
    try {{
        const buffer = new ArrayBuffer(8);
        const view = new Int32Array(buffer);
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 105 error: ${e.message}`);
    }}

    
// ---- fragment 106 ----
    try {{
        const littleEndian = (() => {
          const buffer = new ArrayBuffer(2);
          new DataView(buffer).setInt16(0, 256, true /* littleEndian */);
          // Int16Array uses the platform's endianness.
          return new Int16Array(buffer)[0] === 256;
        })();
        console.log(littleEndian); // true or false
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 106 error: ${e.message}`);
    }}

    
// ---- fragment 107 ----
    try {{
        const buffer = new ArrayBuffer(16);
        const view = new DataView(buffer, 0);

        view.setInt16(1, 42);
        view.getInt16(1); // 42
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 107 error: ${e.message}`);
    }}

    
// ---- fragment 108 ----
    try {{
        registry.register(target, "some value");
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 108 error: ${e.message}`);
    }}

    
// ---- fragment 109 ----
    try {{
        registry.register(theObject, "some value");
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 109 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part11 };
