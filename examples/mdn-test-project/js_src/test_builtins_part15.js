// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 140-149)
// Generated: 2026-06-28

function test_builtins_part15() {
// ---- fragment 140 ----
    try {{
        String.fromCodePoint("_"); // RangeError
        String.fromCodePoint(Infinity); // RangeError
        String.fromCodePoint(-1); // RangeError
        String.fromCodePoint(3.14); // RangeError
        String.fromCodePoint(3e-2); // RangeError
        String.fromCodePoint(NaN); // RangeError
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 140 error: ${e.message}`);
    }}

    
// ---- fragment 141 ----
    try {{
        "foo".normalize("nfc"); // RangeError
        "foo".normalize(" NFC "); // RangeError
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 141 error: ${e.message}`);
    }}

    
// ---- fragment 142 ----
    try {{
        "foo".normalize("NFC"); // 'foo'
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 142 error: ${e.message}`);
    }}

    
// ---- fragment 143 ----
    try {{
        const invalid = new Date("nothing");
        invalid.toISOString(); // RangeError: invalid date
        invalid.toJSON(); // RangeError: invalid date
        JSON.stringify({ date: invalid }); // RangeError: invalid date
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 143 error: ${e.message}`);
    }}

    
// ---- fragment 144 ----
    try {{
        const invalid = new Date("nothing");
        invalid.toString(); // "Invalid Date"
        invalid.getDate(); // NaN
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 144 error: ${e.message}`);
    }}

    
// ---- fragment 145 ----
    try {{
        new Date("05 October 2011 14:48 UTC").toISOString(); // "2011-10-05T14:48:00.000Z"
        new Date(1317826080).toISOString(); // "2011-10-05T14:48:00.000Z"
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 145 error: ${e.message}`);
    }}

    
// ---- fragment 146 ----
    try {{
        (77.1234).toExponential(-1); // RangeError
        (77.1234).toExponential(101); // RangeError

        (2.34).toFixed(-100); // RangeError
        (2.34).toFixed(1001); // RangeError

        (1234.5).toPrecision(-1); // RangeError
        (1234.5).toPrecision(101); // RangeError
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 146 error: ${e.message}`);
    }}

    
// ---- fragment 147 ----
    try {{
        (77.1234).toExponential(4); // 7.7123e+1
        (77.1234).toExponential(2); // 7.71e+1

        (2.34).toFixed(1); // 2.3
        (2.35).toFixed(1); // 2.4 (note that it rounds up in this case)

        (5.123456).toPrecision(5); // 5.1235
        (5.123456).toPrecision(2); // 5.1
        (5.123456).toPrecision(1); // 5
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 147 error: ${e.message}`);
    }}

    
// ---- fragment 148 ----
    try {{
        (42).toString(0);
        (42).toString(1);
        (42).toString(37);
        (42).toString(150);
        // You cannot use a string like this for formatting:
        (12071989).toString("MM-dd-yyyy");
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 148 error: ${e.message}`);
    }}

    
// ---- fragment 149 ----
    try {{
        (42).toString(2); // "101010" (binary)
        (13).toString(8); // "15" (octal)
        (0x42).toString(10); // "66" (decimal)
        (100000).toString(16); // "186a0" (hexadecimal)
    }} catch (e) {{
        console.error(`[test_builtins_part15] fragment 149 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part15 };
