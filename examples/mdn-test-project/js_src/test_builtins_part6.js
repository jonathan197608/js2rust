// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 50-71)
// Generated: 2026-06-30

function test_builtins_part6() {
// ---- fragment 50 ----
try {{
        var escape = 0;
        var str = 0;
        escape(str)
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 50 error: ${e.message}`);
    }}

// ---- fragment 51 ----
try {{
        var str = 0;
        var unescape = 0;
        unescape(str)
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 51 error: ${e.message}`);
    }}

// ---- fragment 52 ----
try {{
        // Create a global property with `var`
        var x = 10;

        function createFunction1() {
          const x = 20;
          return new Function("return x;"); // this `x` refers to global `x`
        }

        function createFunction2() {
          const x = 20;
          function f() {
            return x; // this `x` refers to the local `x` above
          }
          return f;
        }

        const f1 = createFunction1();
        console.log(f1()); // 10
        const f2 = createFunction2();
        console.log(f2()); // 20
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 52 error: ${e.message}`);
    }}

// ---- fragment 53 ----
try {{
        var expression = 0;
        const good = Boolean(expression);
        const good2 = !!expression;
            _ = good;
        _ = good2;
}} catch (e) {{
        console.error(`[test_builtins_part6] fragment 53 error: ${e.message}`);
    }}

// ---- fragment 54 ----
try {{
        var expression = 0;
        const bad = new Boolean(expression); // don't use this!
            _ = bad;
}} catch (e) {{
        console.error(`[test_builtins_part6] fragment 54 error: ${e.message}`);
    }}

// ---- fragment 55 ----
try {{
        if (new Boolean(true)) {
          console.log("This log is printed.");
        }

        if (new Boolean(false)) {
          console.log("This log is ALSO printed.");
        }

        const myFalse = new Boolean(false); // myFalse is a Boolean object (not the primitive value false)
        const g = Boolean(myFalse); // g is true
        const myString = new String("Hello"); // myString is a String object
        const s = Boolean(myString); // s is true
            _ = g;
        _ = s;
}} catch (e) {{
        console.error(`[test_builtins_part6] fragment 55 error: ${e.message}`);
    }}

// ---- fragment 56 ----
try {{
        if ([]) {
          console.log("[] is truthy");
        }
        if ([] == false) {
          console.log("[] == false");
        }
        // [] is truthy
        // [] == false
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 56 error: ${e.message}`);
    }}

// ---- fragment 57 ----
try {{
        const bNoParam = Boolean();
        const bZero = Boolean(0);
        const bNull = Boolean(null);
        const bEmptyString = Boolean("");
        const bfalse = Boolean(false);
            _ = bEmptyString;
        _ = bNoParam;
        _ = bNull;
        _ = bZero;
        _ = bfalse;
}} catch (e) {{
        console.error(`[test_builtins_part6] fragment 57 error: ${e.message}`);
    }}

// ---- fragment 58 ----
try {{
        const btrue = Boolean(true);
        const btrueString = Boolean("true");
        const bfalseString = Boolean("false");
        const bSuLin = Boolean("Su Lin");
        const bArrayProto = Boolean([]);
        const bObjProto = Boolean({});
            _ = bArrayProto;
        _ = bObjProto;
        _ = bSuLin;
        _ = bfalseString;
        _ = btrue;
        _ = btrueString;
}} catch (e) {{
        console.error(`[test_builtins_part6] fragment 58 error: ${e.message}`);
    }}

// ---- fragment 71 ----
try {{
        255; // two-hundred and fifty-five
        255.0; // same number
        255 === 255.0; // true
        255 === 0xff; // true (hexadecimal notation)
        255 === 0b11111111; // true (binary notation)
        255 === 0.255e3; // true (decimal exponential notation)
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 71 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part6 };
