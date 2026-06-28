// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 10 (fragment 0-9)
// Generated: 2026-06-28

function test_statements_part1() {
// ---- fragment 0 ----
    try {{
        function getRectArea(width, height) {
          if (width > 0 && height > 0) {
            return width * height;
          }
          return 0;
        }

        console.log(getRectArea(3, 4));

        console.log(getRectArea(-3, 4));
    }} catch (e) {{
        console.error(`[test_statements_part1] fragment 0 error: ${e.message}`);
    }}

    
// ---- fragment 1 ----
    try {{
        function counter() {
          // Infinite loop
          for (let count = 1; ; count++) {
            console.log(`${count}A`); // Until 5
            if (count === 5) {
              return;
            }
            console.log(`${count}B`); // Until 4
          }
          console.log(`${count}C`); // Never appears
        }

        counter();

        // Logs:
        // 1A
        // 1B
        // 2A
        // 2B
        // 3A
        // 3B
        // 4A
        // 4B
        // 5A
    }} catch (e) {{
        console.error(`[test_statements_part1] fragment 1 error: ${e.message}`);
    }}

    
// ---- fragment 2 ----
    try {{
        function magic() {
          return function calc(x) {
            return x * 42;
          };
        }

        const answer = magic();
        answer(1337); // 56154
    }} catch (e) {{
        console.error(`[test_statements_part1] fragment 2 error: ${e.message}`);
    }}

    
// ---- fragment 3 ----
    try {{
        let i = 0;

        while (i < 6) {
          if (i === 3) {
            break;
          }
          i += 1;
        }

        console.log(i);
    }} catch (e) {{
        console.error(`[test_statements_part1] fragment 3 error: ${e.message}`);
    }}

    
// ---- fragment 4 ----
    try {{
        function testBreak(x) {
          let i = 0;

          while (i < 6) {
            if (i === 3) {
              break;
            }
            i += 1;
          }

          return i * x;
        }
    }} catch (e) {{
        console.error(`[test_statements_part1] fragment 4 error: ${e.message}`);
    }}

    
// ---- fragment 5 ----
    try {{
        const food = "sushi";

        switch (food) {
          case "sushi":
            console.log("Sushi is originally from Japan.");
            break;
          case "pizza":
            console.log("Pizza is originally from Italy.");
            break;
          default:
            console.log("I have never heard of that dish.");
            break;
        }
    }} catch (e) {{
        console.error(`[test_statements_part1] fragment 5 error: ${e.message}`);
    }}

    
// ---- fragment 6 ----
    try {{
        outerBlock: {
          innerBlock: {
            console.log("1");
            break outerBlock; // breaks out of both innerBlock and outerBlock
            console.log(":-("); // skipped
          }
          console.log("2"); // skipped
        }
    }} catch (e) {{
        console.error(`[test_statements_part1] fragment 6 error: ${e.message}`);
    }}

    
// ---- fragment 7 ----
    try {{
        function getRectArea(width, height) {
          if (isNaN(width) || isNaN(height)) {
            throw new Error("Parameter is not a number!");
          }
        }

        try {
          getRectArea(3, "A");
        } catch (e) {
          console.error(e);
        }
    }} catch (e) {{
        console.error(`[test_statements_part1] fragment 7 error: ${e.message}`);
    }}

    
// ---- fragment 8 ----
    try {{
        throw expression;
    }} catch (e) {{
        console.error(`[test_statements_part1] fragment 8 error: ${e.message}`);
    }}

    
// ---- fragment 9 ----
    try {{
        throw error; // Throws a previously defined value (e.g. within a catch block)
        throw new Error("Required"); // Throws a new Error object
    }} catch (e) {{
        console.error(`[test_statements_part1] fragment 9 error: ${e.message}`);
    }}

    
}
module.exports = { test_statements_part1 };
