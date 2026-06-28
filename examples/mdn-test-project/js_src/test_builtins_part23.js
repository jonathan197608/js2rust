// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 8 (fragment 220-227)
// Generated: 2026-06-28

function test_builtins_part23() {
// ---- fragment 220 ----
    try {{
        function getCircularReplacer() {
          const ancestors = [];
          return function (key, value) {
            if (typeof value !== "object" || value === null) {
              return value;
            }
            // `this` is the object that value is contained in,
            // i.e., its direct parent.
            while (ancestors.length > 0 && ancestors.at(-1) !== this) {
              ancestors.pop();
            }
            if (ancestors.includes(value)) {
              return "[Circular]";
            }
            ancestors.push(value);
            return value;
          };
        }

        JSON.stringify(circularReference, getCircularReplacer());
        // {"otherData":123,"myself":"[Circular]"}

        const o = {};
        const notCircularReference = [o, o];
        JSON.stringify(notCircularReference, getCircularReplacer());
        // [{},{}]
    }} catch (e) {{
        console.error(`[test_builtins_part23] fragment 220 error: ${e.message}`);
    }}

    
// ---- fragment 221 ----
    try {{
        "abc".matchAll(/./); // TypeError
        "abc".replaceAll(/./, "f"); // TypeError
    }} catch (e) {{
        console.error(`[test_builtins_part23] fragment 221 error: ${e.message}`);
    }}

    
// ---- fragment 222 ----
    try {{
        [..."abc".matchAll(/./g)]; // [[ "a" ], [ "b" ], [ "c" ]]
        "abc".replaceAll(/./g, "f"); // "fff"

        const existingPattern = /./;
        const newPattern = new RegExp(
          existingPattern.source,
          `${existingPattern.flags}g`,
        );
        "abc".replaceAll(newPattern, "f"); // "fff"
    }} catch (e) {{
        console.error(`[test_builtins_part23] fragment 222 error: ${e.message}`);
    }}

    
// ---- fragment 223 ----
    try {{
        "abc".match(/./); // [ "a" ]
        "abc".replace(/./, "f"); // "fbc"

        [..././[Symbol.matchAll]("abc")]; // [[ "a" ]]
    }} catch (e) {{
        console.error(`[test_builtins_part23] fragment 223 error: ${e.message}`);
    }}

    
// ---- fragment 224 ----
    try {{
        null.foo;
        // TypeError: null has no properties

        undefined.bar;
        // TypeError: undefined has no properties
    }} catch (e) {{
        console.error(`[test_builtins_part23] fragment 224 error: ${e.message}`);
    }}

    
// ---- fragment 225 ----
    try {{
        encodeURI("\uD800");
        // "URIError: malformed URI sequence"

        encodeURI("\uDFFF");
        // "URIError: malformed URI sequence"
    }} catch (e) {{
        console.error(`[test_builtins_part23] fragment 225 error: ${e.message}`);
    }}

    
// ---- fragment 226 ----
    try {{
        encodeURI("\uD800\uDFFF");
        // "%F0%90%8F%BF"
    }} catch (e) {{
        console.error(`[test_builtins_part23] fragment 226 error: ${e.message}`);
    }}

    
// ---- fragment 227 ----
    try {{
        decodeURIComponent("%E0%A4%A");
        // "URIError: malformed URI sequence"
    }} catch (e) {{
        console.error(`[test_builtins_part23] fragment 227 error: ${e.message}`);
    }}

}
module.exports = { testBuiltins };
}
module.exports = { test_builtins_part23 };
