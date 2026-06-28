// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 10 (fragment 20-29)
// Generated: 2026-06-28

function test_statements_part3() {
// ---- fragment 20 ----
    try {{
        const result = /(a+)(b+)(c+)/.exec("aaabcc");
        const [, a, b, c] = result;
        console.log(a, b, c); // "aaa" "b" "cc"
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 20 error: ${e.message}`);
    }}

    
// ---- fragment 21 ----
    try {{
        function calcRectArea(width, height) {
          return width * height;
        }

        console.log(calcRectArea(5, 6));
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 21 error: ${e.message}`);
    }}

    
// ---- fragment 22 ----
    try {{
        console.log(
          `'foo' name ${
            "foo" in globalThis ? "is" : "is not"
          } global. typeof foo is ${typeof foo}`,
        );
        if (false) {
          function foo() {
            return 1;
          }
        }

        // In Chrome:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Firefox:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Safari:
        // 'foo' name is global. typeof foo is function
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 22 error: ${e.message}`);
    }}

    
// ---- fragment 23 ----
    try {{
        console.log(
          `'foo' name ${
            "foo" in globalThis ? "is" : "is not"
          } global. typeof foo is ${typeof foo}`,
        );
        if (true) {
          function foo() {
            return 1;
          }
        }

        // In Chrome:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Firefox:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Safari:
        // 'foo' name is global. typeof foo is function
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 23 error: ${e.message}`);
    }}

    
// ---- fragment 24 ----
    try {{
        "use strict";

        {
          foo(); // Logs "foo"
          function foo() {
            console.log("foo");
          }
        }

        console.log(
          `'foo' name ${
            "foo" in globalThis ? "is" : "is not"
          } global. typeof foo is ${typeof foo}`,
        );
        // 'foo' name is not global. typeof foo is undefined
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 24 error: ${e.message}`);
    }}

    
// ---- fragment 25 ----
    try {{
        hoisted(); // Logs "foo"

        function hoisted() {
          console.log("foo");
        }
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 25 error: ${e.message}`);
    }}

    
// ---- fragment 26 ----
    try {{
        notHoisted(); // TypeError: notHoisted is not a function

        var notHoisted = function () {
          console.log("bar");
        };
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 26 error: ${e.message}`);
    }}

    
// ---- fragment 27 ----
    try {{
        function foo(a) {
          function a() {}
          console.log(typeof a);
        }

        foo(2); // Logs "function"
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 27 error: ${e.message}`);
    }}

    
// ---- fragment 28 ----
    try {{
        function calcSales(unitsA, unitsB, unitsC) {
          return unitsA * 79 + unitsB * 129 + unitsC * 699;
        }
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 28 error: ${e.message}`);
    }}

    
// ---- fragment 29 ----
    try {{
        const array = [1, 2, 3];

        // Assign all array values to 0
        for (let i = 0; i < array.length; array[i++] = 0 /* empty statement */);

        console.log(array);
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 29 error: ${e.message}`);
    }}

    
}
module.exports = { test_statements_part3 };
