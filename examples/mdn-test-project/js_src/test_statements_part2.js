// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 10 (fragment 10-19)
// Generated: 2026-06-28

function test_statements_part2() {
// ---- fragment 10 ----
    try {{
        console.log("Error thrown");
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 10 error: ${e.message}`);
    }}

    
// ---- fragment 11 ----
    try {{
        function isNumeric(x) {
          return typeof x === "number";
        }

        function sum(...values) {
          let total = 0;
          for (const v of values) {
            total += v;
          }
          return total;
        }

        console.log(isNumeric(42));
        console.log(sum(1, 2, 3)); // 6
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 11 error: ${e.message}`);
    }}

    
// ---- fragment 12 ----
    try {{
        console.log("readFile callback example");
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 12 error: ${e.message}`);
    }}

    
// ---- fragment 13 ----
    try {{
        function readFilePromise(path) {
          return path;
        }

        console.log(readFilePromise("foo.txt"));
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 13 error: ${e.message}`);
    }}

    
// ---- fragment 14 ----
    try {{
        const number = 42;

        try {
          number = 99;
        } catch (err) {
          console.log(err);
          // (Note: the exact output may be browser-dependent)
        }

        console.log(number);
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 14 error: ${e.message}`);
    }}

    
// ---- fragment 15 ----
    try {{
        // define MY_FAV as a constant and give it the value 7
        const MY_FAV = 7;

        console.log(`my favorite number is: ${MY_FAV}`);
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 15 error: ${e.message}`);
    }}

    
// ---- fragment 16 ----
    try {{
        const MY_OBJECT = { key: "value" };
        MY_OBJECT = { OTHER_KEY: "value" };
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 16 error: ${e.message}`);
    }}

    
// ---- fragment 17 ----
    try {{
        const MY_OBJECT2 = { key: "value" };
        MY_OBJECT2.key = "otherValue";
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 17 error: ${e.message}`);
    }}

    
// ---- fragment 18 ----
    try {{
        const MY_ARRAY = [];
        MY_ARRAY = ["B"];
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 18 error: ${e.message}`);
    }}

    
// ---- fragment 19 ----
    try {{
        const MY_ARRAY2 = [];
        MY_ARRAY2.push("A"); // ["A"]
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 19 error: ${e.message}`);
    }}

    
}
module.exports = { test_statements_part2 };
