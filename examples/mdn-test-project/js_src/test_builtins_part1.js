// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 0-9)
// Generated: 2026-06-30

function test_builtins_part1() {
// ---- fragment 0 ----
try {{
        const maxNumber = 10 ** 1000; // Max positive number

        if (maxNumber === Infinity) {
          console.log("Let's call it Infinity!");
        }

        console.log(1 / maxNumber);
    }} catch (e) {{
        console.error(`[test_builtins_part1] fragment 0 error: ${e.message}`);
    }}

// ---- fragment 1 ----
try {{
        console.log(Infinity); /* Infinity */
        console.log(Infinity + 1); /* Infinity */
        console.log(10 ** 1000); /* Infinity */
        console.log(Math.log(0)); /* -Infinity */
        console.log(1 / Infinity); /* 0 */
        console.log(1 / 0); /* Infinity */
    }} catch (e) {{
        console.error(`[test_builtins_part1] fragment 1 error: ${e.message}`);
    }}

// ---- fragment 2 ----
try {{
        function sanitize(x) {
          if (isNaN(x)) {
            return NaN;
          }
          return x;
        }

        console.log(sanitize("1"));

        console.log(sanitize("NotANumber"));
    }} catch (e) {{
        console.error(`[test_builtins_part1] fragment 2 error: ${e.message}`);
    }}

// ---- fragment 3 ----
try {{
        NaN === NaN; // false
        Number.NaN === NaN; // false
        isNaN(NaN); // true
        isNaN(Number.NaN); // true
        Number.isNaN(NaN); // true

        function valueIsNaN(v) {
          return v !== v;
        }
        valueIsNaN(1); // false
        valueIsNaN(NaN); // true
        valueIsNaN(Number.NaN); // true
    }} catch (e) {{
        console.error(`[test_builtins_part1] fragment 3 error: ${e.message}`);
    }}

// ---- fragment 4 ----
try {{
        isNaN("hello world"); // true
        Number.isNaN("hello world"); // false
    }} catch (e) {{
        console.error(`[test_builtins_part1] fragment 4 error: ${e.message}`);
    }}

// ---- fragment 5 ----
try {{
        isNaN(1n); // TypeError: Conversion from 'BigInt' to 'number' is not allowed.
        Number.isNaN(1n); // false
    }} catch (e) {{
        console.error(`[test_builtins_part1] fragment 5 error: ${e.message}`);
    }}

// ---- fragment 6 ----
try {{
        const arr = [2, 4, NaN, 12];
        arr.indexOf(NaN); // -1
        arr.includes(NaN); // true

        // Methods accepting a properly defined predicate can always find NaN
        arr.findIndex((n) => Number.isNaN(n)); // 2
    }} catch (e) {{
        console.error(`[test_builtins_part1] fragment 6 error: ${e.message}`);
    }}

// ---- fragment 7 ----
try {{
        const f2b = (x) => new Uint8Array(new Float64Array([x]).buffer);
        const b2f = (x) => new Float64Array(x.buffer)[0];
        // Get a byte representation of NaN
        const n = f2b(NaN);
        const m = f2b(NaN);
        // Change the sign bit, which doesn't matter for NaN
        n[7] += 2 ** 7;
        // n[0] += 2**7; for big endian processors
        const nan2 = b2f(n);
        console.log(nan2); // NaN
        console.log(Object.is(nan2, NaN)); // true
        console.log(f2b(NaN)); // Uint8Array(8) [0, 0, 0, 0, 0, 0, 248, 127]
        console.log(f2b(nan2)); // Uint8Array(8) [0, 0, 0, 0, 0, 0, 248, 255]
        // Change the first bit, which is the least significant bit of the mantissa and doesn't matter for NaN
        m[0] = 1;
        // m[7] = 1; for big endian processors
        const nan3 = b2f(m);
        console.log(nan3); // NaN
        console.log(Object.is(nan3, NaN)); // true
        console.log(f2b(NaN)); // Uint8Array(8) [0, 0, 0, 0, 0, 0, 248, 127]
        console.log(f2b(nan3)); // Uint8Array(8) [1, 0, 0, 0, 0, 0, 248, 127]
    }} catch (e) {{
        console.error(`[test_builtins_part1] fragment 7 error: ${e.message}`);
    }}

// ---- fragment 8 ----
try {{
        NaN ** 0 === 1; // true
    }} catch (e) {{
        console.error(`[test_builtins_part1] fragment 8 error: ${e.message}`);
    }}

// ---- fragment 9 ----
try {{
        function div(x) {
          if (isFinite(1000 / x)) {
            return "Number is NOT Infinity.";
          }
          return "Number is Infinity!";
        }

        console.log(div(0));

        console.log(div(1));
    }} catch (e) {{
        console.error(`[test_builtins_part1] fragment 9 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part1 };
