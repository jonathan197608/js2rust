// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 10-19)
// Generated: 2026-06-28

function test_builtins_part2() {
    var value = 0;
    var string = "0";
// ---- fragment 10 ----
    try {{
        isFinite(value)
    }} catch (e) {{
        console.error(`[test_builtins_part2] fragment 10 error: ${e.message}`);
    }}

    
// ---- fragment 11 ----
    try {{
        isFinite(Infinity); // false
        isFinite(NaN); // false
        isFinite(-Infinity); // false

        isFinite(0); // true
        isFinite(2e64); // true
        isFinite(910); // true

        // Would've been false with the more robust Number.isFinite():
        isFinite(null); // true
        isFinite("0"); // true
    }} catch (e) {{
        console.error(`[test_builtins_part2] fragment 11 error: ${e.message}`);
    }}

    
// ---- fragment 12 ----
    try {{
        function milliseconds(x) {
          if (isNaN(x)) {
            return "Not a Number!";
          }
          return x * 1000;
        }

        console.log(milliseconds("100F"));

        console.log(milliseconds("0.0314E+2"));
    }} catch (e) {{
        console.error(`[test_builtins_part2] fragment 12 error: ${e.message}`);
    }}

    
// ---- fragment 13 ----
    try {{
        isNaN(value)
    }} catch (e) {{
        console.error(`[test_builtins_part2] fragment 13 error: ${e.message}`);
    }}

    
// ---- fragment 14 ----
    try {{
        isNaN(NaN); // true
        isNaN(undefined); // true
        isNaN({}); // true

        isNaN(true); // false
        isNaN(null); // false
        isNaN(37); // false

        // Strings
        isNaN("37"); // false: "37" is converted to the number 37 which is not NaN
        isNaN("37.37"); // false: "37.37" is converted to the number 37.37 which is not NaN
        isNaN("37,5"); // true
        isNaN("123ABC"); // true: Number("123ABC") is NaN
        isNaN(""); // false: the empty string is converted to 0 which is not NaN
        isNaN(" "); // false: a string with spaces is converted to 0 which is not NaN

        // Dates
        isNaN(new Date()); // false; Date objects can be converted to a number (timestamp)
        isNaN(new Date().toString()); // true; the string representation of a Date object cannot be parsed as a number

        // Arrays
        isNaN([]); // false; the primitive representation is "", which coverts to the number 0
        isNaN([1]); // false; the primitive representation is "1"
        isNaN([1, 2]); // true; the primitive representation is "1,2", which cannot be parsed as number
    }} catch (e) {{
        console.error(`[test_builtins_part2] fragment 14 error: ${e.message}`);
    }}

    
// ---- fragment 15 ----
    try {{
        console.log(parseFloat(4.567) * 2.0 * Math.PI);

        console.log(parseFloat("4.567abcdefgh") * 2.0 * Math.PI);

        console.log(parseFloat("abcdefgh") * 2.0 * Math.PI);
    }} catch (e) {{
        console.error(`[test_builtins_part2] fragment 15 error: ${e.message}`);
    }}

    
// ---- fragment 16 ----
    try {{
        parseFloat(string)
    }} catch (e) {{
        console.error(`[test_builtins_part2] fragment 16 error: ${e.message}`);
    }}

    
// ---- fragment 17 ----
    try {{
        parseFloat(3.14);
        parseFloat("3.14");
        parseFloat("  3.14  ");
        parseFloat("314e-2");
        parseFloat("0.0314E+2");
        parseFloat("3.14some non-digit characters");
        parseFloat({
          toString() {
            return "3.14";
          },
        });
    }} catch (e) {{
        console.error(`[test_builtins_part2] fragment 17 error: ${e.message}`);
    }}

    
// ---- fragment 18 ----
    try {{
        parseFloat("FF2");
    }} catch (e) {{
        console.error(`[test_builtins_part2] fragment 18 error: ${e.message}`);
    }}

    
// ---- fragment 19 ----
    try {{
        parseFloat("NaN"); // NaN
    }} catch (e) {{
        console.error(`[test_builtins_part2] fragment 19 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part2 };
