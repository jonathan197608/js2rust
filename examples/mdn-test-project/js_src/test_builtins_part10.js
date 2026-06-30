// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 102-111)
// Generated: 2026-06-30

function test_builtins_part10() {
// ---- fragment 102 ----
try {{
        const s1 = "2 + 2"; // creates a string primitive
        const s2 = new String("2 + 2"); // creates a String object
        console.log(eval(s1)); // returns the number 4
        console.log(eval(s2)); // returns the string "2 + 2"
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 102 error: ${e.message}`);
    }}

// ---- fragment 103 ----
try {{
        var s2 = 0;
        console.log(eval(s2.valueOf())); // returns the number 4
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 103 error: ${e.message}`);
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
        console.error(`[test_builtins_part10] fragment 104 error: ${e.message}`);
    }}

// ---- fragment 105 ----
try {{
        const buffer = new ArrayBuffer(8);
        const view = new Int32Array(buffer);
            _ = view;
}} catch (e) {{
        console.error(`[test_builtins_part10] fragment 105 error: ${e.message}`);
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
        console.error(`[test_builtins_part10] fragment 106 error: ${e.message}`);
    }}

// ---- fragment 107 ----
try {{
        const buffer = new ArrayBuffer(16);
        const view = new DataView(buffer, 0);

        view.setInt16(1, 42);
        view.getInt16(1); // 42
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 107 error: ${e.message}`);
    }}

// ---- fragment 108 ----
try {{
        var registry = 0;
        var target = 0;
        registry.register(target, "some value");
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 108 error: ${e.message}`);
    }}

// ---- fragment 109 ----
try {{
        var registry = 0;
        var theObject = 0;
        registry.register(theObject, "some value");
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 109 error: ${e.message}`);
    }}

// ---- fragment 110 ----
try {{
        const AsyncFunction = async function () {}.constructor;
            _ = AsyncFunction;
}} catch (e) {{
        console.error(`[test_builtins_part10] fragment 110 error: ${e.message}`);
    }}

// ---- fragment 111 ----
try {{
        const regex1 = /ab+c/g;
            _ = regex1;
}} catch (e) {{
        console.error(`[test_builtins_part10] fragment 111 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part10 };
