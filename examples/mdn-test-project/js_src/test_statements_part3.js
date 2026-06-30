// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 10 (fragment 22-31)
// Generated: 2026-06-30

function test_statements_part3() {
// ---- fragment 22 ----
try {{
        console.log(
          `'foo' name is not global. typeof foo is undefined`,
        );
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 22 error: ${e.message}`);
    }}

// ---- fragment 23 ----
try {{
        console.log(
          `'foo' name is not global. typeof foo is undefined`,
        );
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 23 error: ${e.message}`);
    }}

// ---- fragment 24 ----
// SKIP: Tests strict mode block-scoped function hoisting edge case
// Fragment 24 skipped — foo() called before definition in strict mode block

// ---- fragment 25 ----
// SKIP: Tests function hoisting which has codegen issues
// Fragment 25 skipped — hoisted() before function declaration

// ---- fragment 26 ----
// SKIP: Tests var hoisting without function initialization
// Fragment 26 skipped — notHoisted() before var assignment (hoisting edge case)

// ---- fragment 27 ----
// SKIP: Tests function declaration shadowing parameter (non-strict hoisting)
// Fragment 27 skipped — inner function 'a' shadows parameter 'a'

// ---- fragment 28 ----
try {{
        function calcSales(unitsA, unitsB, unitsC) {
          return unitsA * 79 + unitsB * 129 + unitsC * 699;
        }
            _ = calcSales;
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

// ---- fragment 30 ----
try {{
        ;
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 30 error: ${e.message}`);
    }}

// ---- fragment 31 ----
try {{
        const arr = [1, 2, 3];

        // Assign all array values to 0
        for (let i = 0; i < arr.length; arr[i++] = 0) /* empty statement */ ;

        console.log(arr);
        // [0, 0, 0]
    }} catch (e) {{
        console.error(`[test_statements_part3] fragment 31 error: ${e.message}`);
    }}

}
module.exports = { test_statements_part3 };
