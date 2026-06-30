// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 82-91)
// Generated: 2026-06-30

function test_builtins_part8() {
// ---- fragment 82 ----
try {{
        0n === 0; // false
        0n == 0; // true
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 82 error: ${e.message}`);
    }}

// ---- fragment 83 ----
try {{
        1n < 2; // true
        2n > 1; // true
        2 > 2; // false
        2n > 2; // false
        2n >= 2; // true
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 83 error: ${e.message}`);
    }}

// ---- fragment 84 ----
try {{
        const mixed = [4n, 6, -12n, 10, 4, 0, 0n];
        // [4n, 6, -12n, 10, 4, 0, 0n]

        mixed.sort(); // default sorting behavior
        // [ -12n, 0, 0n, 10, 4n, 4, 6 ]

        mixed.sort((a, b) => a - b);
        // won't work since subtraction will not work with mixed types
        // TypeError: can't convert BigInt value to Number value

        // sort with an appropriate numeric comparator
        mixed.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
        // [ -12n, 0, 0n, 4n, 4, 6, 10 ]
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 84 error: ${e.message}`);
    }}

// ---- fragment 85 ----
try {{
        Object(0n) === 0n; // false
        Object(0n) === Object(0n); // false

        const o = Object(0n);
        o === o; // true
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 85 error: ${e.message}`);
    }}

// ---- fragment 86 ----
try {{
        if (0n) {
          console.log("Hello from the if!");
        } else {
          console.log("Hello from the else!");
        }
        // "Hello from the else!"

        0n || 12n; // 12n
        0n && 12n; // 0n
        Boolean(0n); // false
        Boolean(12n); // true
        !12n; // false
        !0n; // true
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 86 error: ${e.message}`);
    }}

// ---- fragment 87 ----
try {{
        BigInt.prototype.toJSON = function () {
          return { $bigint: this.toString() };
        };
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 87 error: ${e.message}`);
    }}

// ---- fragment 88 ----
try {{
        console.log(JSON.stringify({ a: 1n }));
        // {"a":{"$bigint":"1"}}
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 88 error: ${e.message}`);
    }}

// ---- fragment 89 ----
try {{
        const replacer = (key, value) =>
          typeof value === "bigint" ? { $bigint: value.toString() } : value;

        const data = {
          number: 1,
          big: 18014398509481982n,
        };
        const stringified = JSON.stringify(data, replacer);

        console.log(stringified);
        // {"number":1,"big":{"$bigint":"18014398509481982"}}
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 89 error: ${e.message}`);
    }}

// ---- fragment 90 ----
try {{
        const reviver = (key, value) =>
          value !== null &&
          typeof value === "object" &&
          "$bigint" in value &&
          typeof value.$bigint === "string"
            ? BigInt(value.$bigint)
            : value;

        const payload = '{"number":1,"big":{"$bigint":"18014398509481982"}}';
        const parsed = JSON.parse(payload, reviver);

        console.log(parsed);
        // { number: 1, big: 18014398509481982n }
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 90 error: ${e.message}`);
    }}

// ---- fragment 91 ----
try {{
        function isPrime(n) {
          if (n < 2n) {
            return false;
          }
          if (n % 2n === 0n) {
            return n === 2n;
          }
          for (let factor = 3n; factor * factor <= n; factor += 2n) {
            if (n % factor === 0n) {
              return false;
            }
          }
          return true;
        }

        // Takes a BigInt value as an argument, returns nth prime number as a BigInt value
        function nthPrime(nth) {
          let maybePrime = 2n;
          let prime = 0n;

          while (nth >= 0n) {
            if (isPrime(maybePrime)) {
              nth--;
              prime = maybePrime;
            }
            maybePrime++;
          }

          return prime;
        }

        nthPrime(20n);
        // 73n
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 91 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part8 };
