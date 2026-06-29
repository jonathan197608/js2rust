// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 90-99)
// Generated: 2026-06-28

function test_builtins_part10() {
// ---- fragment 90 ----
    try {{
        const reviver = (key, value) =>
          value !== null &&
          typeof value === "object" &&
          "$bigint" in value &&
          typeof value["$bigint"] === "string"
            ? BigInt(value["$bigint"])
            : value;

        const payload = '{"number":1,"big":{"$bigint":"18014398509481982"}}';
        const parsed = JSON.parse(payload, reviver);

        console.log(parsed);
        // { number: 1, big: 18014398509481982n }
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 90 error: ${e.message}`);
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
        console.error(`[test_builtins_part10] fragment 91 error: ${e.message}`);
    }}

    
// ---- fragment 92 ----
    try {{
        function degToRad(degrees) {
          return degrees * (Math.PI / 180);
        }

        function radToDeg(rad) {
          return rad / (Math.PI / 180);
        }
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 92 error: ${e.message}`);
    }}

    
// ---- fragment 93 ----
    try {{
        50 * Math.tan(degToRad(60));
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 93 error: ${e.message}`);
    }}

    
// ---- fragment 94 ----
    try {{
        function random(min, max) {
          const num = Math.floor(Math.random() * (max - min + 1)) + min;
          return num;
        }

        random(1, 10);
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 94 error: ${e.message}`);
    }}

    
// ---- fragment 95 ----
    try {{
        const string1 = "A string primitive";
        const string2 = 'Also a string primitive';
        const string3 = `Yet another string primitive`;
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 95 error: ${e.message}`);
    }}

    
// ---- fragment 96 ----
    try {{
        const string4 = new String("A String object");
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 96 error: ${e.message}`);
    }}

    
// ---- fragment 97 ----
    try {{
        "cat".charAt(1); // gives value "a"
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 97 error: ${e.message}`);
    }}

    
// ---- fragment 98 ----
    try {{
        "cat"[1]; // gives value "a"
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 98 error: ${e.message}`);
    }}

    
// ---- fragment 99 ----
    try {{
        const a = "a";
        const b = "b";
        if (a < b) {
          // true
          console.log(`${a} is less than ${b}`);
        } else if (a > b) {
          console.log(`${a} is greater than ${b}`);
        } else {
          console.log(`${a} and ${b} are equal.`);
        }
    }} catch (e) {{
        console.error(`[test_builtins_part10] fragment 99 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part10 };
