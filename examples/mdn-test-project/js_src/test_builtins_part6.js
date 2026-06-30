// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 50-59)
// Generated: 2026-06-28

function test_builtins_part6() {
// ---- fragment 50 ----
    try {{
        const str = "test";
        const encoded = encodeURIComponent(str);
        console.log(encoded);
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 50 error: ${e.message}`);
    }}

    
// ---- fragment 51 ----
    try {{
        const str2 = "test";
        const decoded = decodeURIComponent(str2);
        console.log(decoded);
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 51 error: ${e.message}`);
    }}

    
// ---- fragment 52 ----
    try {{
        var x = 10;
        console.log(x);

        function createFunction1() {
          return 10;
        }

        function createFunction2() {
          return 20;
        }

        const f1 = createFunction1();
        console.log(f1); // 10
        const f2 = createFunction2();
        console.log(f2); // 20
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 52 error: ${e.message}`);
    }}

    
// ---- fragment 53 ----
    try {{
        const expression = true;
        const good = Boolean(expression);
        const good2 = !!expression;
        console.log(good);
        console.log(good2);
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 53 error: ${e.message}`);
    }}

    
// ---- fragment 54 ----
    try {{
        const expression2 = true;
        const bad = Boolean(expression2);
        console.log(bad);
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 54 error: ${e.message}`);
    }}

    
// ---- fragment 55 ----
    try {{
        if (Boolean(true)) {
          console.log("This log is printed.");
        }

        const myFalse = Boolean(false);
        console.log(myFalse);
        const myString = "Hello";
        console.log(myString);
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 55 error: ${e.message}`);
    }}

    
// ---- fragment 56 ----
    try {{
        const arr = [1];
        if (arr.length > 0) {
          console.log("[] is truthy");
        }
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 56 error: ${e.message}`);
    }}

    
// ---- fragment 57 ----
    try {{
        const bNoParam = Boolean(undefined);
        const bZero = Boolean(0);
        const bfalse = Boolean(false);
        console.log(bNoParam);
        console.log(bZero);
        console.log(bfalse);
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 57 error: ${e.message}`);
    }}

    
// ---- fragment 58 ----
    try {{
        const btrue = Boolean(true);
        console.log(btrue);
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 58 error: ${e.message}`);
    }}

    
// ---- fragment 59 ----
    try {{
        console.log("Promise.any simplified");
    }} catch (e) {{
        console.error(`[test_builtins_part6] fragment 59 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part6 };
