// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 30-39)
// Generated: 2026-06-28

function test_builtins_part4() {
// ---- fragment 30 ----
    try {{
        parseInt("900719925474099267n");
        // 900719925474099300
    }} catch (e) {{
        console.error(`[test_builtins_part4] fragment 30 error: ${e.message}`);
    }}

    
// ---- fragment 31 ----
    try {{
        BigInt("900719925474099267");
        // 900719925474099267n
    }} catch (e) {{
        console.error(`[test_builtins_part4] fragment 31 error: ${e.message}`);
    }}

    
// ---- fragment 32 ----
    try {{
        parseInt("123_456"); // 123
    }} catch (e) {{
        console.error(`[test_builtins_part4] fragment 32 error: ${e.message}`);
    }}

    
// ---- fragment 33 ----
    try {{
        parseInt(null, 36); // 1112745: The string "null" is 1112745 in base 36
        parseInt(undefined, 36); // 86464843759093: The string "undefined" is 86464843759093 in base 36
    }} catch (e) {{
        console.error(`[test_builtins_part4] fragment 33 error: ${e.message}`);
    }}

    
// ---- fragment 34 ----
    try {{
        parseInt(15.99, 10); // 15
        parseInt(-15.1, 10); // -15
    }} catch (e) {{
        console.error(`[test_builtins_part4] fragment 34 error: ${e.message}`);
    }}

    
// ---- fragment 35 ----
    try {{
        parseInt(4.7 * 1e22, 10); // Very large number becomes 4
        parseInt(0.00000000000434, 10); // Very small number becomes 4

        parseInt(0.0000001, 10); // 1
        parseInt(0.000000123, 10); // 1
        parseInt(1e-7, 10); // 1
        parseInt(1000000000000000000000, 10); // 1
        parseInt(123000000000000000000000, 10); // 1
        parseInt(1e21, 10); // 1
    }} catch (e) {{
        console.error(`[test_builtins_part4] fragment 35 error: ${e.message}`);
    }}

    
// ---- fragment 36 ----
    try {{
        decodeURI(encodedURI)
    }} catch (e) {{
        console.error(`[test_builtins_part4] fragment 36 error: ${e.message}`);
    }}

    
// ---- fragment 37 ----
    try {{
        decodeURI(
          "https://developer.mozilla.org/docs/JavaScript%3A%20a_scripting_language",
        );
        // "https://developer.mozilla.org/docs/JavaScript%3A a_scripting_language"

        decodeURIComponent(
          "https://developer.mozilla.org/docs/JavaScript%3A%20a_scripting_language",
        );
        // "https://developer.mozilla.org/docs/JavaScript: a_scripting_language"
    }} catch (e) {{
        console.error(`[test_builtins_part4] fragment 37 error: ${e.message}`);
    }}

    
// ---- fragment 38 ----
    try {{
        try {
          const a = decodeURI("%E0%A4%A");
        } catch (e) {
          console.error(e);
        }

        // URIError: malformed URI sequence
    }} catch (e) {{
        console.error(`[test_builtins_part4] fragment 38 error: ${e.message}`);
    }}

    
// ---- fragment 39 ----
    try {{
        decodeURIComponent(encodedURI)
    }} catch (e) {{
        console.error(`[test_builtins_part4] fragment 39 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part4 };
