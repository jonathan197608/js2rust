// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 80-89)
// Generated: 2026-06-28

function test_builtins_part9() {
// ---- fragment 80 ----
    try {{
        typeof Object(1n) === "object"; // true
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 80 error: ${e.message}`);
    }}

    
// ---- fragment 81 ----
    try {{
        const previousMaxSafe = BigInt(Number.MAX_SAFE_INTEGER); // 9007199254740991n
        const maxPlusOne = previousMaxSafe + 1n; // 9007199254740992n
        const theFuture = previousMaxSafe + 2n; // 9007199254740993n, this works now!
        const prod = previousMaxSafe * 2n; // 18014398509481982n
        const diff = prod - 10n; // 18014398509481972n
        const mod = prod % 10n; // 2n
        const bigN = 2n ** 54n; // 18014398509481984n
        bigN * -1n; // -18014398509481984n
        const expected = 4n / 2n; // 2n
        const truncated = 5n / 2n; // 2n, not 2.5n
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 81 error: ${e.message}`);
    }}

    
// ---- fragment 82 ----
    try {{
        0n === 0; // false
        0n == 0; // true
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 82 error: ${e.message}`);
    }}

    
// ---- fragment 83 ----
    try {{
        1n < 2; // true
        2n > 1; // true
        2 > 2; // false
        2n > 2; // false
        2n >= 2; // true
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 83 error: ${e.message}`);
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
        console.error(`[test_builtins_part9] fragment 84 error: ${e.message}`);
    }}

    
// ---- fragment 85 ----
    try {{
        Object(0n) === 0n; // false
        Object(0n) === Object(0n); // false

        const o = Object(0n);
        o === o; // true
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 85 error: ${e.message}`);
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
        console.error(`[test_builtins_part9] fragment 86 error: ${e.message}`);
    }}

    
// ---- fragment 87 ----
    try {{
        BigInt.prototype.toJSON = function () {
          return { $bigint: this.toString() };
        };
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 87 error: ${e.message}`);
    }}

    
// ---- fragment 88 ----
    try {{
        console.log(JSON.stringify({ a: 1n }));
        // {"a":{"$bigint":"1"}}
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 88 error: ${e.message}`);
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
        console.error(`[test_builtins_part9] fragment 89 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part9 };
