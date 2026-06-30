// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 5 (fragment 223-227)
// Generated: 2026-06-30

function test_builtins_part22() {
// ---- fragment 223 ----
try {{
        "abc".match(/./); // [ "a" ]
        "abc".replace(/./, "f"); // "fbc"

        [..././[Symbol.matchAll]("abc")]; // [[ "a" ]]
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 223 error: ${e.message}`);
    }}

// ---- fragment 224 ----
try {{
        null.foo;
        // TypeError: null has no properties

        undefined.bar;
        // TypeError: undefined has no properties
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 224 error: ${e.message}`);
    }}

// ---- fragment 225 ----
try {{
        encodeURI("\uD800");
        // "URIError: malformed URI sequence"

        encodeURI("\uDFFF");
        // "URIError: malformed URI sequence"
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 225 error: ${e.message}`);
    }}

// ---- fragment 226 ----
try {{
        encodeURI("\uD800\uDFFF");
        // "%F0%90%8F%BF"
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 226 error: ${e.message}`);
    }}

// ---- fragment 227 ----
try {{
        decodeURIComponent("%E0%A4%A");
        // "URIError: malformed URI sequence"
    }} catch (e) {{
        console.error(`[test_builtins_part22] fragment 227 error: ${e.message}`);
    }}

}
module.exports = { testBuiltins };

}
module.exports = { test_builtins_part22 };
