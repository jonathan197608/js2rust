// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 40-49)
// Generated: 2026-06-30

function test_builtins_part5() {
// ---- fragment 40 ----
try {{
        try {
          const a = decodeURIComponent("%E0%A4%A");
        } catch (e) {
          console.error(e);
        }

        // URIError: malformed URI sequence
    }} catch (e) {{
        console.error(`[test_builtins_part5] fragment 40 error: ${e.message}`);
    }}

// ---- fragment 41 ----
try {{
        function decodeQueryParam(p) {
          return decodeURIComponent(p.replace(/\+/g, " "));
        }

        decodeQueryParam("search+query%20%28correct%29");
        // 'search query (correct)'
    }} catch (e) {{
        console.error(`[test_builtins_part5] fragment 41 error: ${e.message}`);
    }}

// ---- fragment 42 ----
try {{
        encodeURI(uri)
    }} catch (e) {{
        console.error(`[test_builtins_part5] fragment 42 error: ${e.message}`);
    }}

// ---- fragment 43 ----
try {{
        const set1 = ";/?:@&=+$,#"; // Reserved Characters
        const set2 = "-.!~*'()"; // Unreserved Marks
        const set3 = "ABC abc 123"; // Alphanumeric Characters + Space

        console.log(encodeURI(set1)); // ;/?:@&=+$,#
        console.log(encodeURI(set2)); // -.!~*'()
        console.log(encodeURI(set3)); // ABC%20abc%20123 (the space gets encoded as %20)

        console.log(encodeURIComponent(set1)); // %3B%2C%2F%3F%3A%40%26%3D%2B%24%23
        console.log(encodeURIComponent(set2)); // -.!~*'()
        console.log(encodeURIComponent(set3)); // ABC%20abc%20123 (the space gets encoded as %20)
    }} catch (e) {{
        console.error(`[test_builtins_part5] fragment 43 error: ${e.message}`);
    }}

// ---- fragment 44 ----
try {{
        // High-low pair OK
        encodeURI("\uD800\uDFFF"); // "%F0%90%8F%BF"

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURI("\uD800");

        // Lone low-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURI("\uDFFF");
    }} catch (e) {{
        console.error(`[test_builtins_part5] fragment 44 error: ${e.message}`);
    }}

// ---- fragment 45 ----
try {{
        function encodeRFC3986URI(str) {
          return encodeURI(str)
            .replace(/%5B/g, "[")
            .replace(/%5D/g, "]")
            .replace(
              /[!'()*]/g,
              (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
            );
        }
    }} catch (e) {{
        console.error(`[test_builtins_part5] fragment 45 error: ${e.message}`);
    }}

// ---- fragment 46 ----
try {{
        encodeURIComponent(uriComponent)
    }} catch (e) {{
        console.error(`[test_builtins_part5] fragment 46 error: ${e.message}`);
    }}

// ---- fragment 47 ----
try {{
        const fileName = "my file(2).txt";
        const header = `Content-Disposition: attachment; filename*=UTF-8''${encodeRFC5987ValueChars(
          fileName,
        )}`;

        console.log(header);
        // "Content-Disposition: attachment; filename*=UTF-8''my%20file%282%29.txt"

        function encodeRFC5987ValueChars(str) {
          return (
            encodeURIComponent(str)
              // The following creates the sequences %27 %28 %29 %2A (Note that
              // the valid encoding of "*" is %2A, which necessitates calling
              // toUpperCase() to properly encode). Although RFC3986 reserves "!",
              // RFC5987 does not, so we do not need to escape it.
              .replace(
                /['()*]/g,
                (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
              )
              // The following are not required for percent-encoding per RFC5987,
              // so we can allow for a little better readability over the wire: |`^
              .replace(/%(7C|60|5E)/g, (str, hex) =>
                String.fromCharCode(parseInt(hex, 16)),
              )
          );
        }
    }} catch (e) {{
        console.error(`[test_builtins_part5] fragment 47 error: ${e.message}`);
    }}

// ---- fragment 48 ----
try {{
        function encodeRFC3986URIComponent(str) {
          return encodeURIComponent(str).replace(
            /[!'()*]/g,
            (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
          );
        }
    }} catch (e) {{
        console.error(`[test_builtins_part5] fragment 48 error: ${e.message}`);
    }}

// ---- fragment 49 ----
try {{
        // High-low pair OK
        encodeURIComponent("\uD800\uDFFF"); // "%F0%90%8F%BF"

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURIComponent("\uD800");

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURIComponent("\uDFFF");
    }} catch (e) {{
        console.error(`[test_builtins_part5] fragment 49 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part5 };
