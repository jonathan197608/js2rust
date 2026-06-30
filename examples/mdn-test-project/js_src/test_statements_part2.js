// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 10 (fragment 11-21)
// Generated: 2026-06-30

function test_statements_part2() {
// ---- fragment 11 ----
try {{
        function isNumeric(x) {
          return ["number", "bigint"].includes(typeof x);
        }

        function sum(...values) {
          if (!values.every(isNumeric)) {
            throw new TypeError("Can only add numbers");
          }
          return values.reduce((a, b) => a + b);
        }

        console.log(sum(1, 2, 3)); // 6
        try {
          sum("1", "2");
        } catch (e) {
          console.error(e); // TypeError: Can only add numbers
        }
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 11 error: ${e.message}`);
    }}

// ---- fragment 12 ----
try {{
        var readFile = 0;
        readFile("foo.txt", (err, data) => {
          if (err) {
            throw err;
          }
          console.log(data);
        });
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 12 error: ${e.message}`);
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
            _ = MY_FAV;
}} catch (e) {{
        console.error(`[test_statements_part2] fragment 15 error: ${e.message}`);
    }}

// ---- fragment 16 ----
try {{
        var OTHER_KEY = 0;
        var key = 0;
        const MY_OBJECT = { key: "value" };
        MY_OBJECT = { OTHER_KEY: "value" };
            _ = MY_OBJECT;
        _ = OTHER_KEY;
        _ = key;
}} catch (e) {{
        console.error(`[test_statements_part2] fragment 16 error: ${e.message}`);
    }}

// ---- fragment 17 ----
try {{
        var MY_OBJECT = 0;
        MY_OBJECT.key = "otherValue";
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
        var MY_ARRAY = 0;
        MY_ARRAY.push("A"); // ["A"]
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 19 error: ${e.message}`);
    }}

// ---- fragment 20 ----
try {{
        const result = /(a+)(b+)(c+)/.exec("aaabcc");
        const [, a, b, c] = result;
        console.log(a, b, c); // "aaa" "b" "cc"
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 20 error: ${e.message}`);
    }}

// ---- fragment 21 ----
try {{
        function calcRectArea(width, height) {
          return width * height;
        }

        console.log(calcRectArea(5, 6));
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 21 error: ${e.message}`);
    }}

}
module.exports = { test_statements_part2 };
