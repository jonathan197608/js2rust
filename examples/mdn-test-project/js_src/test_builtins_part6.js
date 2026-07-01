// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 50-59)
// Generated: 2026-06-28

function test_builtins_part6() {
// ---- fragment 50 ----
    try {{
        escape(str)
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 50 error: ${e.message}`);
    }}

    
// ---- fragment 51 ----
    try {{
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
        const good = Boolean(expression);
        const good2 = !!expression;
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 53 error: ${e.message}`);
    }}

    
// ---- fragment 54 ----
    try {{
        const bad = new Boolean(expression); // don't use this!
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
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 58 error: ${e.message}`);
    }}

    
// ---- fragment 59 ----
    try {{
        Promise.any([Promise.reject(new Error("some error"))]).catch((e) => {
          console.log(e instanceof AggregateError); // true
          console.log(e.message); // "All Promises rejected"
          console.log(e.name); // "AggregateError"
          console.log(e.errors); // [ Error: "some error" ]
        });
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 59 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part6 };
