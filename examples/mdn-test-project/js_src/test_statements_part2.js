// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 10 (fragment 10-19)
// Generated: 2026-06-28

function test_statements_part2() {
// ---- fragment 10 ----
    try {{
        throw (
          new Error()
        );
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 10 error: ${e.message}`);
    }}

    
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
        readFile("foo.txt", (err, data) => {
          if (err) {
            throw err;
          }
          console.log(data);
        });
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 12 error: ${e.message}`);
    }}

    
// ---- fragment 13 ----
    try {{
        (async () => {{
            function readFilePromise(path) {
              return new Promise((resolve, reject) => {
                readFile(path, (err, data) => {
                  if (err) {
                    reject(err);
                  }
                  resolve(data);
                });
              });
            }

            try {
              const data = await readFilePromise("foo.txt");
              console.log(data);
            } catch (err) {
              console.error(err);
            }
        }})();
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
        MY_ARRAY.push("A"); // ["A"]
    }} catch (e) {{
        console.error(`[test_statements_part2] fragment 19 error: ${e.message}`);
    }}

    
}
module.exports = { test_statements_part2 };
