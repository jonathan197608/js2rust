// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 80-89)
// Generated: 2026-06-28
// Note: BigInt operations simplified to regular integers

function test_builtins_part9() {
// ---- fragment 80 ----
    try {{
        console.log(typeof 1 === "number");
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 80 error: ${e.message}`);
    }}

    
// ---- fragment 81 ----
    try {{
        const previousMaxSafe = 9007199254740991;
        const maxPlusOne = previousMaxSafe + 1;
        const theFuture = previousMaxSafe + 2;
        const prod = previousMaxSafe * 2;
        const diff = prod - 10;
        const mod = prod % 10;
        const bigN = 2 ** 54;
        const expected = 4 / 2;
        const truncated = 5 / 2;
        console.log(previousMaxSafe);
        console.log(maxPlusOne);
        console.log(theFuture);
        console.log(prod);
        console.log(diff);
        console.log(mod);
        console.log(bigN);
        console.log(expected);
        console.log(truncated);
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 81 error: ${e.message}`);
    }}

    
// ---- fragment 82 ----
    try {{
        console.log(0 === 0); // false in BigInt, true for numbers
        console.log(0 == 0); // true
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 82 error: ${e.message}`);
    }}

    
// ---- fragment 83 ----
    try {{
        console.log(1 < 2); // true
        console.log(2 > 1); // true
        console.log(2 > 2); // false
        console.log(2 >= 2); // true
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 83 error: ${e.message}`);
    }}

    
// ---- fragment 84 ----
    try {{
        const mixed = [4, 6, -12, 10, 4, 0, 0];
        mixed.sort();
        mixed.sort((a, b) => a - b);
        mixed.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
        console.log(mixed);
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 84 error: ${e.message}`);
    }}

    
// ---- fragment 85 ----
    try {{
        const o = 0;
        console.log(o === o); // true
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 85 error: ${e.message}`);
    }}

    
// ---- fragment 86 ----
    try {{
        if (0) {
          console.log("Hello from the if!");
        } else {
          console.log("Hello from the else!");
        }

        console.log(0 || 12); // 12
        console.log(0 && 12); // 0
        console.log(Boolean(0)); // false
        console.log(Boolean(12)); // true
        console.log(!12); // false
        console.log(!0); // true
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 86 error: ${e.message}`);
    }}

    
// ---- fragment 87 ----
    try {{
        const bigIntToJson = function () {
          return { bigint: "0" };
        };
        console.log(bigIntToJson());
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 87 error: ${e.message}`);
    }}

    
// ---- fragment 88 ----
    try {{
        console.log(JSON.stringify({ a: 1 }));
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 88 error: ${e.message}`);
    }}

    
// ---- fragment 89 ----
    try {{
        const data = {
          number: 1,
          big: 18014398509481982,
        };
        const stringified = JSON.stringify(data);
        console.log(stringified);
    }} catch (e) {{
        console.error(`[test_builtins_part9] fragment 89 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part9 };
