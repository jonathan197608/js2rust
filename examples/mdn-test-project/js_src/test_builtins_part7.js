// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 72-81)
// Generated: 2026-06-30

function test_builtins_part7() {
// ---- fragment 72 ----
try {{
        Number("123"); // returns the number 123
        Number("123") === 123; // true

        Number("unicorn"); // NaN
        Number(undefined); // NaN
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 72 error: ${e.message}`);
    }}

// ---- fragment 73 ----
try {{
        new Int32Array([1.1, 1.9, -1.1, -1.9]); // Int32Array(4) [ 1, 1, -1, -1 ]

        new Int8Array([257, -257]); // Int8Array(2) [ 1, -1 ]
        // 257 = 0001 0000 0001
        //     =      0000 0001 (mod 2^8)
        //     = 1
        // -257 = 1110 1111 1111
        //      =      1111 1111 (mod 2^8)
        //      = -1 (as signed integer)

        new Uint8Array([257, -257]); // Uint8Array(2) [ 1, 255 ]
        // -257 = 1110 1111 1111
        //      =      1111 1111 (mod 2^8)
        //      = 255 (as unsigned integer)
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 73 error: ${e.message}`);
    }}

// ---- fragment 74 ----
try {{
        const biggestNum = Number.MAX_VALUE;
        const smallestNum = Number.MIN_VALUE;
        const infiniteNum = Number.POSITIVE_INFINITY;
        const negInfiniteNum = Number.NEGATIVE_INFINITY;
        const notANum = Number.NaN;
            _ = biggestNum;
        _ = infiniteNum;
        _ = negInfiniteNum;
        _ = notANum;
        _ = smallestNum;
}} catch (e) {{
        console.error(`[test_builtins_part7] fragment 74 error: ${e.message}`);
    }}

// ---- fragment 75 ----
try {{
        const biggestInt = Number.MAX_SAFE_INTEGER; // (2**53 - 1) => 9007199254740991
        const smallestInt = Number.MIN_SAFE_INTEGER; // -(2**53 - 1) => -9007199254740991
            _ = biggestInt;
        _ = smallestInt;
}} catch (e) {{
        console.error(`[test_builtins_part7] fragment 75 error: ${e.message}`);
    }}

// ---- fragment 76 ----
try {{
        const d = new Date("1995-12-17T03:24:00");
        console.log(Number(d));
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 76 error: ${e.message}`);
    }}

// ---- fragment 77 ----
try {{
        Number("123"); // 123
        Number("123") === 123; // true
        Number("12.3"); // 12.3
        Number("12.00"); // 12
        Number("123e-1"); // 12.3
        Number(""); // 0
        Number(null); // 0
        Number("0x11"); // 17
        Number("0b11"); // 3
        Number("0o11"); // 9
        Number("foo"); // NaN
        Number("100a"); // NaN
        Number("-Infinity"); // -Infinity
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 77 error: ${e.message}`);
    }}

// ---- fragment 78 ----
try {{
        const previouslyMaxSafeInteger = 9007199254740991n;

        const alsoHuge = BigInt(9007199254740991);
        // 9007199254740991n

        const hugeString = BigInt("9007199254740991");
        // 9007199254740991n

        const hugeHex = BigInt("0x1fffffffffffff");
        // 9007199254740991n

        const hugeOctal = BigInt("0o377777777777777777");
        // 9007199254740991n

        const hugeBin = BigInt(
          "0b11111111111111111111111111111111111111111111111111111",
        );
        // 9007199254740991n
            _ = alsoHuge;
        _ = hugeBin;
        _ = hugeHex;
        _ = hugeOctal;
        _ = hugeString;
        _ = previouslyMaxSafeInteger;
}} catch (e) {{
        console.error(`[test_builtins_part7] fragment 78 error: ${e.message}`);
    }}

// ---- fragment 79 ----
try {{
        typeof 1n === "bigint"; // true
        typeof BigInt("1") === "bigint"; // true
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 79 error: ${e.message}`);
    }}

// ---- fragment 80 ----
try {{
        typeof Object(1n) === "object"; // true
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 80 error: ${e.message}`);
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
            _ = diff;
        _ = expected;
        _ = maxPlusOne;
        _ = mod;
        _ = theFuture;
        _ = truncated;
}} catch (e) {{
        console.error(`[test_builtins_part7] fragment 81 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part7 };
