// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 213-222)
// Generated: 2026-06-30

function test_builtins_part21() {
// ---- fragment 213 ----
try {{
        Warning: SyntaxError: Using //@ to indicate sourceURL pragmas is deprecated. Use //# instead

        Warning: SyntaxError: Using //@ to indicate sourceMappingURL pragmas is deprecated. Use //# instead
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 213 error: ${e.message}`);
    }}

// ---- fragment 214 ----
try {{
        Object.defineProperty({}, "key", 1);
        // TypeError: 1 is not a non-null object

        Object.defineProperty({}, "key", null);
        // TypeError: null is not a non-null object
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 214 error: ${e.message}`);
    }}

// ---- fragment 215 ----
try {{
        Object.defineProperty({}, "key", { value: "foo", writable: false });
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 215 error: ${e.message}`);
    }}

// ---- fragment 216 ----
try {{
        Object.setPrototypeOf(Object.prototype, {});
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 216 error: ${e.message}`);
    }}

// ---- fragment 217 ----
try {{
        const obj = {};
        Object.preventExtensions(obj);
        Object.setPrototypeOf(obj, {});
        // TypeError: can't set prototype of this object
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 217 error: ${e.message}`);
    }}

// ---- fragment 218 ----
try {{
        const circularReference = { otherData: 123 };
        circularReference.myself = circularReference;
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 218 error: ${e.message}`);
    }}

// ---- fragment 219 ----
try {{
        JSON.stringify(circularReference);
        // TypeError: cyclic object value
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 219 error: ${e.message}`);
    }}

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
        console.error(`[test_builtins_part21] fragment 220 error: ${e.message}`);
    }}

// ---- fragment 221 ----
try {{
        "abc".matchAll(/./); // TypeError
        "abc".replaceAll(/./, "f"); // TypeError
    }} catch (e) {{
        console.error(`[test_builtins_part21] fragment 221 error: ${e.message}`);
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
        console.error(`[test_builtins_part21] fragment 222 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part21 };
