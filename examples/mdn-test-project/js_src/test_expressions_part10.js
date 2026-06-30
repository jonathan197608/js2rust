// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 90-99)
// Generated: 2026-06-30

function test_expressions_part10() {
// ---- fragment 90 ----
try {{
        "1" != 1; // false
        1 != "1"; // false
        0 != false; // false
        0 != null; // true
        0 != undefined; // true
        0 != !!null; // false, look at Logical NOT operator
        0 != !!undefined; // false, look at Logical NOT operator
        null != undefined; // false

        const number1 = new Number(3);
        const number2 = new Number(3);
        number1 != 3; // false
        number1 != number2; // true
    }} catch (e) {{
        console.error(`[test_expressions_part10] fragment 90 error: ${e.message}`);
    }}

// ---- fragment 91 ----
try {{
        var key = 0;
        const object1 = {
          key: "value",
        };

        const object2 = {
          key: "value",
        };

        console.log(object1 != object2); // true
        console.log(object1 != object1); // false
            _ = key;
}} catch (e) {{
        console.error(`[test_expressions_part10] fragment 91 error: ${e.message}`);
    }}

// ---- fragment 92 ----
try {{
        console.log(1 === 1);

        console.log("hello" === "hello");

        console.log("1" === 1);

        console.log(0 === false);
    }} catch (e) {{
        console.error(`[test_expressions_part10] fragment 92 error: ${e.message}`);
    }}

// ---- fragment 93 ----
try {{
        var x = 1;
        var y = 2;
        x === y
    }} catch (e) {{
        console.error(`[test_expressions_part10] fragment 93 error: ${e.message}`);
    }}

// ---- fragment 94 ----
try {{
        "hello" === "hello"; // true
        "hello" === "hola"; // false

        3 === 3; // true
        3 === 4; // false

        true === true; // true
        true === false; // false

        null === null; // true
    }} catch (e) {{
        console.error(`[test_expressions_part10] fragment 94 error: ${e.message}`);
    }}

// ---- fragment 95 ----
try {{
        "3" === 3; // false
        true === 1; // false
        null === undefined; // false
        3 === new Number(3); // false
    }} catch (e) {{
        console.error(`[test_expressions_part10] fragment 95 error: ${e.message}`);
    }}

// ---- fragment 96 ----
try {{
        var key = 0;
        const object1 = {
          key: "value",
        };

        const object2 = {
          key: "value",
        };

        console.log(object1 === object2); // false
        console.log(object1 === object1); // true
            _ = key;
}} catch (e) {{
        console.error(`[test_expressions_part10] fragment 96 error: ${e.message}`);
    }}

// ---- fragment 97 ----
try {{
        console.log(1 !== 1);

        console.log("hello" !== "hello");

        console.log("1" !== 1);

        console.log(0 !== false);
    }} catch (e) {{
        console.error(`[test_expressions_part10] fragment 97 error: ${e.message}`);
    }}

// ---- fragment 98 ----
try {{
        var x = 1;
        var y = 2;
        x !== y
    }} catch (e) {{
        console.error(`[test_expressions_part10] fragment 98 error: ${e.message}`);
    }}

// ---- fragment 99 ----
try {{
        var x = 1;
        var y = 2;
        x !== y;

        !(x === y);
    }} catch (e) {{
        console.error(`[test_expressions_part10] fragment 99 error: ${e.message}`);
    }}

}
module.exports = { test_expressions_part10 };
