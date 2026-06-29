// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 70-79)
// Generated: 2026-06-28

function test_builtins_part8() {
// ---- fragment 70 ----
    try {{
        // instanceof and error property access (e.message, e.name, e.stack)
        // are not supported in Zig transpilation; fragment omitted
        console.log("URIError test omitted");
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 70 error: ${e.message}`);
    }}

    
// ---- fragment 71 ----
    try {{
        255; // two-hundred and fifty-five
        255.0; // same number
        255 === 255.0; // true
        255 === 0xff; // true (hexadecimal notation)
        255 === 0b11111111; // true (binary notation)
        255 === 0.255e3; // true (decimal exponential notation)
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 71 error: ${e.message}`);
    }}

    
// ---- fragment 72 ----
    try {{
        Number("123"); // returns the number 123
        Number("123") === 123; // true

        Number("unicorn"); // NaN
        Number(undefined); // NaN
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 72 error: ${e.message}`);
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
        console.error(`[test_builtins_part8] fragment 73 error: ${e.message}`);
    }}

    
// ---- fragment 74 ----
    try {{
        const biggestNum = Number.MAX_VALUE;
        const smallestNum = Number.MIN_VALUE;
        const infiniteNum = Number.POSITIVE_INFINITY;
        const negInfiniteNum = Number.NEGATIVE_INFINITY;
        const notANum = Number.NaN;
        console.log(biggestNum);
        console.log(smallestNum);
        console.log(infiniteNum);
        console.log(negInfiniteNum);
        console.log(notANum);
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 74 error: ${e.message}`);
    }}

    
// ---- fragment 75 ----
    try {{
        const biggestInt = Number.MAX_SAFE_INTEGER; // (2**53 - 1) => 9007199254740991
        const smallestInt = Number.MIN_SAFE_INTEGER; // -(2**53 - 1) => -9007199254740991
        console.log(biggestInt);
        console.log(smallestInt);
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 75 error: ${e.message}`);
    }}

    
// ---- fragment 76 ----
    try {{
        const d = new Date("1995-12-17T03:24:00");
        console.log(Number(d));
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 76 error: ${e.message}`);
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
        console.error(`[test_builtins_part8] fragment 77 error: ${e.message}`);
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
        console.log(previouslyMaxSafeInteger);
        console.log(alsoHuge);
        console.log(hugeString);
        console.log(hugeHex);
        console.log(hugeOctal);
        console.log(hugeBin);
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 78 error: ${e.message}`);
    }}

    
// ---- fragment 79 ----
    try {{
        typeof 1n === "bigint"; // true
        typeof BigInt("1") === "bigint"; // true
    }} catch (e) {{
        console.error(`[test_builtins_part8] fragment 79 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part8 };
