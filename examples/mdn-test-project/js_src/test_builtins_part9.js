// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 92-101)
// Generated: 2026-06-30

function test_builtins_part9() {
// ---- fragment 92 ----
try {{
        function degToRad(degrees) {
          return degrees * (Math.PI / 180);
        }

        function radToDeg(rad) {
          return rad / (Math.PI / 180);
        }
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 92 error: ${e.message}`);
    }}

// ---- fragment 93 ----
try {{
        50 * Math.tan(degToRad(60));
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 93 error: ${e.message}`);
    }}

// ---- fragment 94 ----
try {{
        function random(min, max) {
          const num = Math.floor(Math.random() * (max - min + 1)) + min;
          return num;
        }

        random(1, 10);
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 94 error: ${e.message}`);
    }}

// ---- fragment 95 ----
try {{
        const string1 = "A string primitive";
        const string2 = 'Also a string primitive';
        const string3 = `Yet another string primitive`;
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 95 error: ${e.message}`);
    }}

// ---- fragment 96 ----
try {{
        const string4 = new String("A String object");
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 96 error: ${e.message}`);
    }}

// ---- fragment 97 ----
try {{
        "cat".charAt(1); // gives value "a"
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 97 error: ${e.message}`);
    }}

// ---- fragment 98 ----
try {{
        "cat"[1]; // gives value "a"
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 98 error: ${e.message}`);
    }}

// ---- fragment 99 ----
try {{
        const a = "a";
        const b = "b";
        if (a < b) {
          // true
          console.log(`${a} is less than ${b}`);
        } else if (a > b) {
          console.log(`${a} is greater than ${b}`);
        } else {
          console.log(`${a} and ${b} are equal.`);
        }
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 99 error: ${e.message}`);
    }}

// ---- fragment 100 ----
try {{
        function areEqualCaseInsensitive(str1, str2) {
          return str1.toUpperCase() === str2.toUpperCase();
        }
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 100 error: ${e.message}`);
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
        console.error(`[test_builtins_part9] fragment 101 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part9 };
