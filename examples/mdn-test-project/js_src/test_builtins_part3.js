// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 20-29)
// Generated: 2026-06-30

function test_builtins_part3() {
// ---- fragment 20 ----
try {{
        parseFloat("1.7976931348623159e+308"); // Infinity
        parseFloat("-1.7976931348623159e+308"); // -Infinity
    }} catch (e) {{
        console.error(`[test_builtins_part3] fragment 20 error: ${e.message}`);
    }}

// ---- fragment 21 ----
try {{
        parseFloat("Infinity"); // Infinity
        parseFloat("-Infinity"); // -Infinity
    }} catch (e) {{
        console.error(`[test_builtins_part3] fragment 21 error: ${e.message}`);
    }}

// ---- fragment 22 ----
try {{
        parseFloat(900719925474099267n); // 900719925474099300
        parseFloat("900719925474099267n"); // 900719925474099300
    }} catch (e) {{
        console.error(`[test_builtins_part3] fragment 22 error: ${e.message}`);
    }}

// ---- fragment 23 ----
try {{
        BigInt("900719925474099267");
        // 900719925474099267n
    }} catch (e) {{
        console.error(`[test_builtins_part3] fragment 23 error: ${e.message}`);
    }}

// ---- fragment 24 ----
try {{
        console.log(parseInt("123"));
        // 123 (default base-10)
        console.log(parseInt("123", 10));
        // 123 (explicitly specify base-10)
        console.log(parseInt("   123 "));
        // 123 (whitespace is ignored)
        console.log(parseInt("077"));
        // 77 (leading zeros are ignored)
        console.log(parseInt("1.9"));
        // 1 (decimal part is truncated)
        console.log(parseInt("ff", 16));
        // 255 (lower-case hexadecimal)
        console.log(parseInt("0xFF", 16));
        // 255 (upper-case hexadecimal with "0x" prefix)
        console.log(parseInt("xyz"));
        // NaN (input can't be converted to an integer)
    }} catch (e) {{
        console.error(`[test_builtins_part3] fragment 24 error: ${e.message}`);
    }}

// ---- fragment 25 ----
try {{
        var radix = 0;
        var string = "0";
        parseInt(string)
        parseInt(string, radix)
    }} catch (e) {{
        console.error(`[test_builtins_part3] fragment 25 error: ${e.message}`);
    }}

// ---- fragment 26 ----
try {{
        parseInt("0xF", 16);
        parseInt("F", 16);
        parseInt("17", 8);
        parseInt("015", 10);
        parseInt("15,123", 10);
        parseInt("FXX123", 16);
        parseInt("1111", 2);
        parseInt("15 * 3", 10);
        parseInt("15e2", 10);
        parseInt("15px", 10);
        parseInt("12", 13);
    }} catch (e) {{
        console.error(`[test_builtins_part3] fragment 26 error: ${e.message}`);
    }}

// ---- fragment 27 ----
try {{
        parseInt("Hello", 8); // Not a number at all
        parseInt("546", 2); // Digits other than 0 or 1 are invalid for binary radix
    }} catch (e) {{
        console.error(`[test_builtins_part3] fragment 27 error: ${e.message}`);
    }}

// ---- fragment 28 ----
try {{
        parseInt("-F", 16);
        parseInt("-0F", 16);
        parseInt("-0XF", 16);
        parseInt("-17", 8);
        parseInt("-15", 10);
        parseInt("-1111", 2);
        parseInt("-15e1", 10);
        parseInt("-12", 13);
    }} catch (e) {{
        console.error(`[test_builtins_part3] fragment 28 error: ${e.message}`);
    }}

// ---- fragment 29 ----
try {{
        parseInt("0e0", 16);
    }} catch (e) {{
        console.error(`[test_builtins_part3] fragment 29 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part3 };
