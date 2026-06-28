// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 228
// Generated: 2026-06-28

function testBuiltins() {
    // ---- fragment 0 ----
    try {{
        const maxNumber = 10 ** 1000; // Max positive number

        if (maxNumber === Infinity) {
          console.log("Let's call it Infinity!");
        }

        console.log(1 / maxNumber);
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 0 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 1 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 2 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 3 error: ${e.message}`);
    }}

    // ---- fragment 4 ----
    try {{
        isNaN("hello world"); // true
        Number.isNaN("hello world"); // false
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 4 error: ${e.message}`);
    }}

    // ---- fragment 5 ----
    try {{
        isNaN(1n); // TypeError: Conversion from 'BigInt' to 'number' is not allowed.
        Number.isNaN(1n); // false
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 5 error: ${e.message}`);
    }}

    // ---- fragment 6 ----
    try {{
        const arr = [2, 4, NaN, 12];
        arr.indexOf(NaN); // -1
        arr.includes(NaN); // true

        // Methods accepting a properly defined predicate can always find NaN
        arr.findIndex((n) => Number.isNaN(n)); // 2
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 6 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 7 error: ${e.message}`);
    }}

    // ---- fragment 8 ----
    try {{
        NaN ** 0 === 1; // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 8 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 9 error: ${e.message}`);
    }}

    // ---- fragment 10 ----
    try {{
        isFinite(value)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 10 error: ${e.message}`);
    }}

    // ---- fragment 11 ----
    try {{
        isFinite(Infinity); // false
        isFinite(NaN); // false
        isFinite(-Infinity); // false

        isFinite(0); // true
        isFinite(2e64); // true
        isFinite(910); // true

        // Would've been false with the more robust Number.isFinite():
        isFinite(null); // true
        isFinite("0"); // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 11 error: ${e.message}`);
    }}

    // ---- fragment 12 ----
    try {{
        function milliseconds(x) {
          if (isNaN(x)) {
            return "Not a Number!";
          }
          return x * 1000;
        }

        console.log(milliseconds("100F"));

        console.log(milliseconds("0.0314E+2"));
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 12 error: ${e.message}`);
    }}

    // ---- fragment 13 ----
    try {{
        isNaN(value)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 13 error: ${e.message}`);
    }}

    // ---- fragment 14 ----
    try {{
        isNaN(NaN); // true
        isNaN(undefined); // true
        isNaN({}); // true

        isNaN(true); // false
        isNaN(null); // false
        isNaN(37); // false

        // Strings
        isNaN("37"); // false: "37" is converted to the number 37 which is not NaN
        isNaN("37.37"); // false: "37.37" is converted to the number 37.37 which is not NaN
        isNaN("37,5"); // true
        isNaN("123ABC"); // true: Number("123ABC") is NaN
        isNaN(""); // false: the empty string is converted to 0 which is not NaN
        isNaN(" "); // false: a string with spaces is converted to 0 which is not NaN

        // Dates
        isNaN(new Date()); // false; Date objects can be converted to a number (timestamp)
        isNaN(new Date().toString()); // true; the string representation of a Date object cannot be parsed as a number

        // Arrays
        isNaN([]); // false; the primitive representation is "", which coverts to the number 0
        isNaN([1]); // false; the primitive representation is "1"
        isNaN([1, 2]); // true; the primitive representation is "1,2", which cannot be parsed as number
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 14 error: ${e.message}`);
    }}

    // ---- fragment 15 ----
    try {{
        function circumference(r) {
          return parseFloat(r) * 2.0 * Math.PI;
        }

        console.log(circumference(4.567));

        console.log(circumference("4.567abcdefgh"));

        console.log(circumference("abcdefgh"));
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 15 error: ${e.message}`);
    }}

    // ---- fragment 16 ----
    try {{
        parseFloat(string)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 16 error: ${e.message}`);
    }}

    // ---- fragment 17 ----
    try {{
        parseFloat(3.14);
        parseFloat("3.14");
        parseFloat("  3.14  ");
        parseFloat("314e-2");
        parseFloat("0.0314E+2");
        parseFloat("3.14some non-digit characters");
        parseFloat({
          toString() {
            return "3.14";
          },
        });
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 17 error: ${e.message}`);
    }}

    // ---- fragment 18 ----
    try {{
        parseFloat("FF2");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 18 error: ${e.message}`);
    }}

    // ---- fragment 19 ----
    try {{
        parseFloat("NaN"); // NaN
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 19 error: ${e.message}`);
    }}

    // ---- fragment 20 ----
    try {{
        parseFloat("1.7976931348623159e+308"); // Infinity
        parseFloat("-1.7976931348623159e+308"); // -Infinity
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 20 error: ${e.message}`);
    }}

    // ---- fragment 21 ----
    try {{
        parseFloat("Infinity"); // Infinity
        parseFloat("-Infinity"); // -Infinity
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 21 error: ${e.message}`);
    }}

    // ---- fragment 22 ----
    try {{
        parseFloat(900719925474099267n); // 900719925474099300
        parseFloat("900719925474099267n"); // 900719925474099300
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 22 error: ${e.message}`);
    }}

    // ---- fragment 23 ----
    try {{
        BigInt("900719925474099267");
        // 900719925474099267n
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 23 error: ${e.message}`);
    }}

    // ---- fragment 24 ----
    try {{
        console.log(parseInt("123"));
        // 123 (default base-10)
        console.log(parseInt("123", 10));
        // 123 (explicitly specify base-10)
        console.log(parseInt("   123 "));
        // 123 (whitespace is ignored)
        console.log(parseInt("077"));
        // 77 (leading zeros are ignored)
        console.log(parseInt("1.9"));
        // 1 (decimal part is truncated)
        console.log(parseInt("ff", 16));
        // 255 (lower-case hexadecimal)
        console.log(parseInt("0xFF", 16));
        // 255 (upper-case hexadecimal with "0x" prefix)
        console.log(parseInt("xyz"));
        // NaN (input can't be converted to an integer)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 24 error: ${e.message}`);
    }}

    // ---- fragment 25 ----
    try {{
        parseInt(string)
        parseInt(string, radix)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 25 error: ${e.message}`);
    }}

    // ---- fragment 26 ----
    try {{
        parseInt("0xF", 16);
        parseInt("F", 16);
        parseInt("17", 8);
        parseInt("015", 10);
        parseInt("15,123", 10);
        parseInt("FXX123", 16);
        parseInt("1111", 2);
        parseInt("15 * 3", 10);
        parseInt("15e2", 10);
        parseInt("15px", 10);
        parseInt("12", 13);
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 26 error: ${e.message}`);
    }}

    // ---- fragment 27 ----
    try {{
        parseInt("Hello", 8); // Not a number at all
        parseInt("546", 2); // Digits other than 0 or 1 are invalid for binary radix
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 27 error: ${e.message}`);
    }}

    // ---- fragment 28 ----
    try {{
        parseInt("-F", 16);
        parseInt("-0F", 16);
        parseInt("-0XF", 16);
        parseInt("-17", 8);
        parseInt("-15", 10);
        parseInt("-1111", 2);
        parseInt("-15e1", 10);
        parseInt("-12", 13);
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 28 error: ${e.message}`);
    }}

    // ---- fragment 29 ----
    try {{
        parseInt("0e0", 16);
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 29 error: ${e.message}`);
    }}

    // ---- fragment 30 ----
    try {{
        parseInt("900719925474099267n");
        // 900719925474099300
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 30 error: ${e.message}`);
    }}

    // ---- fragment 31 ----
    try {{
        BigInt("900719925474099267");
        // 900719925474099267n
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 31 error: ${e.message}`);
    }}

    // ---- fragment 32 ----
    try {{
        parseInt("123_456"); // 123
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 32 error: ${e.message}`);
    }}

    // ---- fragment 33 ----
    try {{
        parseInt(null, 36); // 1112745: The string "null" is 1112745 in base 36
        parseInt(undefined, 36); // 86464843759093: The string "undefined" is 86464843759093 in base 36
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 33 error: ${e.message}`);
    }}

    // ---- fragment 34 ----
    try {{
        parseInt(15.99, 10); // 15
        parseInt(-15.1, 10); // -15
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 34 error: ${e.message}`);
    }}

    // ---- fragment 35 ----
    try {{
        parseInt(4.7 * 1e22, 10); // Very large number becomes 4
        parseInt(0.00000000000434, 10); // Very small number becomes 4

        parseInt(0.0000001, 10); // 1
        parseInt(0.000000123, 10); // 1
        parseInt(1e-7, 10); // 1
        parseInt(1000000000000000000000, 10); // 1
        parseInt(123000000000000000000000, 10); // 1
        parseInt(1e21, 10); // 1
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 35 error: ${e.message}`);
    }}

    // ---- fragment 36 ----
    try {{
        decodeURI(encodedURI)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 36 error: ${e.message}`);
    }}

    // ---- fragment 37 ----
    try {{
        decodeURI(
          "https://developer.mozilla.org/docs/JavaScript%3A%20a_scripting_language",
        );
        // "https://developer.mozilla.org/docs/JavaScript%3A a_scripting_language"

        decodeURIComponent(
          "https://developer.mozilla.org/docs/JavaScript%3A%20a_scripting_language",
        );
        // "https://developer.mozilla.org/docs/JavaScript: a_scripting_language"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 37 error: ${e.message}`);
    }}

    // ---- fragment 38 ----
    try {{
        try {
          const a = decodeURI("%E0%A4%A");
        } catch (e) {
          console.error(e);
        }

        // URIError: malformed URI sequence
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 38 error: ${e.message}`);
    }}

    // ---- fragment 39 ----
    try {{
        decodeURIComponent(encodedURI)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 39 error: ${e.message}`);
    }}

    // ---- fragment 40 ----
    try {{
        try {
          const a = decodeURIComponent("%E0%A4%A");
        } catch (e) {
          console.error(e);
        }

        // URIError: malformed URI sequence
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 40 error: ${e.message}`);
    }}

    // ---- fragment 41 ----
    try {{
        function decodeQueryParam(p) {
          return decodeURIComponent(p.replace(/\+/g, " "));
        }

        decodeQueryParam("search+query%20%28correct%29");
        // 'search query (correct)'
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 41 error: ${e.message}`);
    }}

    // ---- fragment 42 ----
    try {{
        encodeURI(uri)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 42 error: ${e.message}`);
    }}

    // ---- fragment 43 ----
    try {{
        const set1 = ";/?:@&=+$,#"; // Reserved Characters
        const set2 = "-.!~*'()"; // Unreserved Marks
        const set3 = "ABC abc 123"; // Alphanumeric Characters + Space

        console.log(encodeURI(set1)); // ;/?:@&=+$,#
        console.log(encodeURI(set2)); // -.!~*'()
        console.log(encodeURI(set3)); // ABC%20abc%20123 (the space gets encoded as %20)

        console.log(encodeURIComponent(set1)); // %3B%2C%2F%3F%3A%40%26%3D%2B%24%23
        console.log(encodeURIComponent(set2)); // -.!~*'()
        console.log(encodeURIComponent(set3)); // ABC%20abc%20123 (the space gets encoded as %20)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 43 error: ${e.message}`);
    }}

    // ---- fragment 44 ----
    try {{
        // High-low pair OK
        encodeURI("\uD800\uDFFF"); // "%F0%90%8F%BF"

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURI("\uD800");

        // Lone low-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURI("\uDFFF");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 44 error: ${e.message}`);
    }}

    // ---- fragment 45 ----
    try {{
        function encodeRFC3986URI(str) {
          return encodeURI(str)
            .replace(/%5B/g, "[")
            .replace(/%5D/g, "]")
            .replace(
              /[!'()*]/g,
              (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
            );
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 45 error: ${e.message}`);
    }}

    // ---- fragment 46 ----
    try {{
        encodeURIComponent(uriComponent)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 46 error: ${e.message}`);
    }}

    // ---- fragment 47 ----
    try {{
        const fileName = "my file(2).txt";
        const header = `Content-Disposition: attachment; filename*=UTF-8''${encodeRFC5987ValueChars(
          fileName,
        )}`;

        console.log(header);
        // "Content-Disposition: attachment; filename*=UTF-8''my%20file%282%29.txt"

        function encodeRFC5987ValueChars(str) {
          return (
            encodeURIComponent(str)
              // The following creates the sequences %27 %28 %29 %2A (Note that
              // the valid encoding of "*" is %2A, which necessitates calling
              // toUpperCase() to properly encode). Although RFC3986 reserves "!",
              // RFC5987 does not, so we do not need to escape it.
              .replace(
                /['()*]/g,
                (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
              )
              // The following are not required for percent-encoding per RFC5987,
              // so we can allow for a little better readability over the wire: |`^
              .replace(/%(7C|60|5E)/g, (str, hex) =>
                String.fromCharCode(parseInt(hex, 16)),
              )
          );
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 47 error: ${e.message}`);
    }}

    // ---- fragment 48 ----
    try {{
        function encodeRFC3986URIComponent(str) {
          return encodeURIComponent(str).replace(
            /[!'()*]/g,
            (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
          );
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 48 error: ${e.message}`);
    }}

    // ---- fragment 49 ----
    try {{
        // High-low pair OK
        encodeURIComponent("\uD800\uDFFF"); // "%F0%90%8F%BF"

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURIComponent("\uD800");

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURIComponent("\uDFFF");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 49 error: ${e.message}`);
    }}

    // ---- fragment 50 ----
    try {{
        escape(str)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 50 error: ${e.message}`);
    }}

    // ---- fragment 51 ----
    try {{
        unescape(str)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 51 error: ${e.message}`);
    }}

    // ---- fragment 52 ----
    try {{
        // Create a global property with `var`
        var x = 10;

        function createFunction1() {
          const x = 20;
          return new Function("return x;"); // this `x` refers to global `x`
        }

        function createFunction2() {
          const x = 20;
          function f() {
            return x; // this `x` refers to the local `x` above
          }
          return f;
        }

        const f1 = createFunction1();
        console.log(f1()); // 10
        const f2 = createFunction2();
        console.log(f2()); // 20
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 52 error: ${e.message}`);
    }}

    // ---- fragment 53 ----
    try {{
        const good = Boolean(expression);
        const good2 = !!expression;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 53 error: ${e.message}`);
    }}

    // ---- fragment 54 ----
    try {{
        const bad = new Boolean(expression); // don't use this!
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 54 error: ${e.message}`);
    }}

    // ---- fragment 55 ----
    try {{
        if (new Boolean(true)) {
          console.log("This log is printed.");
        }

        if (new Boolean(false)) {
          console.log("This log is ALSO printed.");
        }

        const myFalse = new Boolean(false); // myFalse is a Boolean object (not the primitive value false)
        const g = Boolean(myFalse); // g is true
        const myString = new String("Hello"); // myString is a String object
        const s = Boolean(myString); // s is true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 55 error: ${e.message}`);
    }}

    // ---- fragment 56 ----
    try {{
        if ([]) {
          console.log("[] is truthy");
        }
        if ([] == false) {
          console.log("[] == false");
        }
        // [] is truthy
        // [] == false
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 56 error: ${e.message}`);
    }}

    // ---- fragment 57 ----
    try {{
        const bNoParam = Boolean();
        const bZero = Boolean(0);
        const bNull = Boolean(null);
        const bEmptyString = Boolean("");
        const bfalse = Boolean(false);
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 57 error: ${e.message}`);
    }}

    // ---- fragment 58 ----
    try {{
        const btrue = Boolean(true);
        const btrueString = Boolean("true");
        const bfalseString = Boolean("false");
        const bSuLin = Boolean("Su Lin");
        const bArrayProto = Boolean([]);
        const bObjProto = Boolean({});
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 58 error: ${e.message}`);
    }}

    // ---- fragment 59 ----
    try {{
        Promise.any([Promise.reject(new Error("some error"))]).catch((e) => {
          console.log(e instanceof AggregateError); // true
          console.log(e.message); // "All Promises rejected"
          console.log(e.name); // "AggregateError"
          console.log(e.errors); // [ Error: "some error" ]
        });
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 59 error: ${e.message}`);
    }}

    // ---- fragment 60 ----
    try {{
        try {
          throw new AggregateError([new Error("some error")], "Hello");
        } catch (e) {
          console.log(e instanceof AggregateError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "AggregateError"
          console.log(e.errors); // [ Error: "some error" ]
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 60 error: ${e.message}`);
    }}

    // ---- fragment 61 ----
    try {{
        try {
          throw new EvalError("Hello");
        } catch (e) {
          console.log(e instanceof EvalError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "EvalError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 61 error: ${e.message}`);
    }}

    // ---- fragment 62 ----
    try {{
        try {
          let a = undefinedVariable;
        } catch (e) {
          console.log(e instanceof ReferenceError); // true
          console.log(e.message); // "undefinedVariable is not defined"
          console.log(e.name); // "ReferenceError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 62 error: ${e.message}`);
    }}

    // ---- fragment 63 ----
    try {{
        try {
          throw new ReferenceError("Hello");
        } catch (e) {
          console.log(e instanceof ReferenceError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "ReferenceError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 63 error: ${e.message}`);
    }}

    // ---- fragment 64 ----
    try {{
        try {
          throw new SuppressedError(
            new Error("New error"),
            new Error("Original error"),
            "Hello",
          );
        } catch (e) {
          console.log(e instanceof SuppressedError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "SuppressedError"
          console.log(e.error); // Error: "New error"
          console.log(e.suppressed); // Error: "Original error"
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 64 error: ${e.message}`);
    }}

    // ---- fragment 65 ----
    try {{
        try {
          eval("hoo bar");
        } catch (e) {
          console.log(e instanceof SyntaxError); // true
          console.log(e.message);
          console.log(e.name); // "SyntaxError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 65 error: ${e.message}`);
    }}

    // ---- fragment 66 ----
    try {{
        try {
          throw new SyntaxError("Hello");
        } catch (e) {
          console.log(e instanceof SyntaxError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "SyntaxError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 66 error: ${e.message}`);
    }}

    // ---- fragment 67 ----
    try {{
        try {
          null.f();
        } catch (e) {
          console.log(e instanceof TypeError); // true
          console.log(e.message); // "null has no properties"
          console.log(e.name); // "TypeError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 67 error: ${e.message}`);
    }}

    // ---- fragment 68 ----
    try {{
        try {
          throw new TypeError("Hello");
        } catch (e) {
          console.log(e instanceof TypeError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "TypeError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 68 error: ${e.message}`);
    }}

    // ---- fragment 69 ----
    try {{
        try {
          decodeURIComponent("%");
        } catch (e) {
          console.log(e instanceof URIError); // true
          console.log(e.message); // "malformed URI sequence"
          console.log(e.name); // "URIError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 69 error: ${e.message}`);
    }}

    // ---- fragment 70 ----
    try {{
        try {
          throw new URIError("Hello");
        } catch (e) {
          console.log(e instanceof URIError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "URIError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 70 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 71 error: ${e.message}`);
    }}

    // ---- fragment 72 ----
    try {{
        Number("123"); // returns the number 123
        Number("123") === 123; // true

        Number("unicorn"); // NaN
        Number(undefined); // NaN
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 72 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 73 error: ${e.message}`);
    }}

    // ---- fragment 74 ----
    try {{
        const biggestNum = Number.MAX_VALUE;
        const smallestNum = Number.MIN_VALUE;
        const infiniteNum = Number.POSITIVE_INFINITY;
        const negInfiniteNum = Number.NEGATIVE_INFINITY;
        const notANum = Number.NaN;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 74 error: ${e.message}`);
    }}

    // ---- fragment 75 ----
    try {{
        const biggestInt = Number.MAX_SAFE_INTEGER; // (2**53 - 1) => 9007199254740991
        const smallestInt = Number.MIN_SAFE_INTEGER; // -(2**53 - 1) => -9007199254740991
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 75 error: ${e.message}`);
    }}

    // ---- fragment 76 ----
    try {{
        const d = new Date("1995-12-17T03:24:00");
        console.log(Number(d));
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 76 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 77 error: ${e.message}`);
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
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 78 error: ${e.message}`);
    }}

    // ---- fragment 79 ----
    try {{
        typeof 1n === "bigint"; // true
        typeof BigInt("1") === "bigint"; // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 79 error: ${e.message}`);
    }}

    // ---- fragment 80 ----
    try {{
        typeof Object(1n) === "object"; // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 80 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 81 error: ${e.message}`);
    }}

    // ---- fragment 82 ----
    try {{
        0n === 0; // false
        0n == 0; // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 82 error: ${e.message}`);
    }}

    // ---- fragment 83 ----
    try {{
        1n < 2; // true
        2n > 1; // true
        2 > 2; // false
        2n > 2; // false
        2n >= 2; // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 83 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 84 error: ${e.message}`);
    }}

    // ---- fragment 85 ----
    try {{
        Object(0n) === 0n; // false
        Object(0n) === Object(0n); // false

        const o = Object(0n);
        o === o; // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 85 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 86 error: ${e.message}`);
    }}

    // ---- fragment 87 ----
    try {{
        BigInt.prototype.toJSON = function () {
          return { $bigint: this.toString() };
        };
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 87 error: ${e.message}`);
    }}

    // ---- fragment 88 ----
    try {{
        console.log(JSON.stringify({ a: 1n }));
        // {"a":{"$bigint":"1"}}
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 88 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 89 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 90 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 91 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 92 error: ${e.message}`);
    }}

    // ---- fragment 93 ----
    try {{
        50 * Math.tan(degToRad(60));
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 93 error: ${e.message}`);
    }}

    // ---- fragment 94 ----
    try {{
        function random(min, max) {
          const num = Math.floor(Math.random() * (max - min + 1)) + min;
          return num;
        }

        random(1, 10);
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 94 error: ${e.message}`);
    }}

    // ---- fragment 95 ----
    try {{
        const string1 = "A string primitive";
        const string2 = 'Also a string primitive';
        const string3 = `Yet another string primitive`;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 95 error: ${e.message}`);
    }}

    // ---- fragment 96 ----
    try {{
        const string4 = new String("A String object");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 96 error: ${e.message}`);
    }}

    // ---- fragment 97 ----
    try {{
        "cat".charAt(1); // gives value "a"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 97 error: ${e.message}`);
    }}

    // ---- fragment 98 ----
    try {{
        "cat"[1]; // gives value "a"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 98 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 99 error: ${e.message}`);
    }}

    // ---- fragment 100 ----
    try {{
        function areEqualCaseInsensitive(str1, str2) {
          return str1.toUpperCase() === str2.toUpperCase();
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 100 error: ${e.message}`);
    }}

    // ---- fragment 101 ----
    try {{
        const strPrim = "foo"; // A literal is a string primitive
        const strPrim2 = String(1); // Coerced into the string primitive "1"
        const strPrim3 = String(true); // Coerced into the string primitive "true"
        const strObj = new String(strPrim); // String with new returns a string wrapper object.

        console.log(typeof strPrim); // "string"
        console.log(typeof strPrim2); // "string"
        console.log(typeof strPrim3); // "string"
        console.log(typeof strObj); // "object"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 101 error: ${e.message}`);
    }}

    // ---- fragment 102 ----
    try {{
        const s1 = "2 + 2"; // creates a string primitive
        const s2 = new String("2 + 2"); // creates a String object
        console.log(eval(s1)); // returns the number 4
        console.log(eval(s2)); // returns the string "2 + 2"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 102 error: ${e.message}`);
    }}

    // ---- fragment 103 ----
    try {{
        console.log(eval(s2.valueOf())); // returns the number 4
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 103 error: ${e.message}`);
    }}

    // ---- fragment 104 ----
    try {{
        // You cannot access properties on null or undefined

        const nullVar = null;
        nullVar.toString(); // TypeError: Cannot read properties of null
        String(nullVar); // "null"

        const undefinedVar = undefined;
        undefinedVar.toString(); // TypeError: Cannot read properties of undefined
        String(undefinedVar); // "undefined"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 104 error: ${e.message}`);
    }}

    // ---- fragment 105 ----
    try {{
        const buffer = new ArrayBuffer(8);
        const view = new Int32Array(buffer);
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 105 error: ${e.message}`);
    }}

    // ---- fragment 106 ----
    try {{
        const littleEndian = (() => {
          const buffer = new ArrayBuffer(2);
          new DataView(buffer).setInt16(0, 256, true /* littleEndian */);
          // Int16Array uses the platform's endianness.
          return new Int16Array(buffer)[0] === 256;
        })();
        console.log(littleEndian); // true or false
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 106 error: ${e.message}`);
    }}

    // ---- fragment 107 ----
    try {{
        const buffer = new ArrayBuffer(16);
        const view = new DataView(buffer, 0);

        view.setInt16(1, 42);
        view.getInt16(1); // 42
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 107 error: ${e.message}`);
    }}

    // ---- fragment 108 ----
    try {{
        registry.register(target, "some value");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 108 error: ${e.message}`);
    }}

    // ---- fragment 109 ----
    try {{
        registry.register(theObject, "some value");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 109 error: ${e.message}`);
    }}

    // ---- fragment 110 ----
    try {{
        const AsyncFunction = async function () {}.constructor;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 110 error: ${e.message}`);
    }}

    // ---- fragment 111 ----
    try {{
        const regex1 = /ab+c/g;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 111 error: ${e.message}`);
    }}

    // ---- fragment 112 ----
    try {{
        const regex2 = new RegExp("ab+c", "g");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 112 error: ${e.message}`);
    }}

    // ---- fragment 113 ----
    try {{
        /[\s-9]/.test("-"); // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 113 error: ${e.message}`);
    }}

    // ---- fragment 114 ----
    try {{
        const r1 = /\p{Lowercase_Letter}/iu;
        const r2 = /[^\P{Lowercase_Letter}]/iu;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 114 error: ${e.message}`);
    }}

    // ---- fragment 115 ----
    try {{
        function isHexadecimal(str) {
          return /^[0-9A-F]+$/i.test(str);
        }

        isHexadecimal("2F3"); // true
        isHexadecimal("beef"); // true
        isHexadecimal("undefined"); // false
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 115 error: ${e.message}`);
    }}

    // ---- fragment 116 ----
    try {{
        function getLineTerminators(str) {
          return str.match(/[\r\n\u2028\u2029\q{\r\n}]/gv);
        }

        getLineTerminators(`
        A poem\r
        Is split\r\n
        Into many
        Stanzas
        `); // [ '\r', '\r\n', '\n' ]
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 116 error: ${e.message}`);
    }}

    // ---- fragment 117 ----
    try {{
        function splitWords(str) {
          return str.split(/\s+/);
        }

        splitWords(`Look at the stars
        Look  how they\tshine for you`);
        // ['Look', 'at', 'the', 'stars', 'Look', 'how', 'they', 'shine', 'for', 'you']
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 117 error: ${e.message}`);
    }}

    // ---- fragment 118 ----
    try {{
        /[\c0]/.test("\x10"); // true
        /[\c_]/.test("\x1f"); // true
        /[\c*]/.test("\\"); // true
        /\c/.test("\\c"); // true
        /\c0/.test("\\c0"); // true (the \c0 syntax is only supported in character classes)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 118 error: ${e.message}`);
    }}

    // ---- fragment 119 ----
    try {{
        const pattern = /a\nb/;
        const string = `a
        b`;
        console.log(pattern.test(string)); // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 119 error: ${e.message}`);
    }}

    // ---- fragment 120 ----
    try {{
        /a|ab/.exec("abc"); // ['a']
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 120 error: ${e.message}`);
    }}

    // ---- fragment 121 ----
    try {{
        /(?:(a)|(ab))(?:(c)|(bc))/.exec("abc"); // ['abc', 'a', undefined, undefined, 'bc']
        // Not ['abc', undefined, 'ab', 'c', undefined]
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 121 error: ${e.message}`);
    }}

    // ---- fragment 122 ----
    try {{
        function isImage(filename) {
          return /\.(?:png|jpe?g|webp|avif|gif)$/i.test(filename);
        }

        isImage("image.png"); // true
        isImage("image.jpg"); // true
        isImage("image.pdf"); // false
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 122 error: ${e.message}`);
    }}

    // ---- fragment 123 ----
    try {{
        function removeTrailingSlash(url) {
          return url.replace(/\/$/, "");
        }

        removeTrailingSlash("https://example.com/"); // "https://example.com"
        removeTrailingSlash("https://example.com/docs/"); // "https://example.com/docs"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 123 error: ${e.message}`);
    }}

    // ---- fragment 124 ----
    try {{
        function isImage(filename) {
          return /\.(?:png|jpe?g|webp|avif|gif)$/i.test(filename);
        }

        isImage("image.png"); // true
        isImage("image.jpg"); // true
        isImage("image.pdf"); // false
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 124 error: ${e.message}`);
    }}

    // ---- fragment 125 ----
    try {{
        function isValidIdentifier(str) {
          return /^[$_\p{ID_Start}][$_\p{ID_Continue}]*$/u.test(str);
        }

        isValidIdentifier("foo"); // true
        isValidIdentifier("$1"); // true
        isValidIdentifier("1foo"); // false
        isValidIdentifier("  foo  "); // false
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 125 error: ${e.message}`);
    }}

    // ---- fragment 126 ----
    try {{
        const variables = ["foo", "foo:bar", "  foo  "];

        function toAssignment(key) {
          if (isValidIdentifier(key)) {
            return `globalThis.${key} = undefined;`;
          }
          // JSON.stringify() escapes quotes and other special characters
          return `globalThis[${JSON.stringify(key)}] = undefined;`;
        }

        const statements = variables.map(toAssignment).join("\n");

        console.log(statements);
        // globalThis.foo = undefined;
        // globalThis["foo:bar"] = undefined;
        // globalThis["  foo  "] = undefined;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 126 error: ${e.message}`);
    }}

    // ---- fragment 127 ----
    try {{
        /\k/.test("k"); // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 127 error: ${e.message}`);
    }}

    // ---- fragment 128 ----
    try {{
        const re = /a{1, 3}/;
        re.test("aa"); // false
        re.test("a{1, 3}"); // true
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 128 error: ${e.message}`);
    }}

    // ---- fragment 129 ----
    try {{
        /[ab]*/.exec("aba"); // ['aba']
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 129 error: ${e.message}`);
    }}

    // ---- fragment 130 ----
    try {{
        /a*/.exec("aaa"); // ['aaa']; the entire input is consumed
        /a*?/.exec("aaa"); // ['']; it's possible to consume no characters and still match successfully
        /^a*?$/.exec("aaa"); // ['aaa']; it's not possible to consume fewer characters and still match successfully
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 130 error: ${e.message}`);
    }}

    // ---- fragment 131 ----
    try {{
        /a*?$/.exec("aaa"); // ['aaa']; the match already succeeds at the first character, so the regex never attempts to start matching at the second character
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 131 error: ${e.message}`);
    }}

    // ---- fragment 132 ----
    try {{
        /[ab]+[abc]c/.exec("abbc"); // ['abbc']
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 132 error: ${e.message}`);
    }}

    // ---- fragment 133 ----
    try {{
        /(?=a)?b/.test("b"); // true; the lookahead is matched 0 time
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 133 error: ${e.message}`);
    }}

    // ---- fragment 134 ----
    try {{
        function countParagraphs(str) {
          return str.match(/(?:\r?\n){2,}/g).length + 1;
        }

        countParagraphs(`
        Paragraph 1

        Paragraph 2
        Containing some line breaks, but still the same paragraph

        Another paragraph
        `); // 3
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 134 error: ${e.message}`);
    }}

    // ---- fragment 135 ----
    try {{
        // finding all the letters of a text
        const story = "It's the Cheshire Cat: now I shall have somebody to talk to.";

        // Most explicit form
        story.match(/\p{General_Category=Letter}/gu);

        // It is not mandatory to use the property name for General categories
        story.match(/\p{Letter}/gu);

        // This is equivalent (short alias):
        story.match(/\p{L}/gu);

        // This is also equivalent (conjunction of all the subcategories using short aliases)
        story.match(/\p{Lu}|\p{Ll}|\p{Lt}|\p{Lm}|\p{Lo}/gu);
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 135 error: ${e.message}`);
    }}

    // ---- fragment 136 ----
    try {{
        // Ù¢ is the digit 2 in Arabic-Indic notation
        // while it is predominantly written within the Arabic script
        // it can also be written in the Thaana script

        "Ù¢".match(/\p{Script=Thaana}/u);
        // null as Thaana is not the predominant script

        "Ù¢".match(/\p{Script_Extensions=Thaana}/u);
        // ["Ù¢", index: 0, input: "Ù¢", groups: undefined]
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 136 error: ${e.message}`);
    }}

    // ---- fragment 137 ----
    try {{
        /\ba/.exec("abc");
        /c\b/.exec("abc");

        /\B /.exec(" abc");
        / \B/.exec("abc ");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 137 error: ${e.message}`);
    }}

    // ---- fragment 138 ----
    try {{
        function hasThanks(str) {
          return /\b(thanks|thank you)\b/i.test(str);
        }

        hasThanks("Thanks! You helped me a lot."); // true
        hasThanks("Just want to say thank you for all your work."); // true
        hasThanks("Thanksgiving is around the corner."); // false
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 138 error: ${e.message}`);
    }}

    // ---- fragment 139 ----
    try {{
        var a = 2;
        try {
          throw new Error();
        } catch (a) {
          var a = 1; // This 1 is assigned to the caught `a`, not the outer `a`.
        }
        console.log(a); // 2

        try {
          throw new Error();
          // Note: identifier changed to `err` to avoid conflict with
          // the inner declaration of `a`.
        } catch (err) {
          var a = 1; // This 1 is assigned to the upper-scope `a`.
        }
        console.log(a); // 1
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 139 error: ${e.message}`);
    }}

    // ---- fragment 140 ----
    try {{
        String.fromCodePoint("_"); // RangeError
        String.fromCodePoint(Infinity); // RangeError
        String.fromCodePoint(-1); // RangeError
        String.fromCodePoint(3.14); // RangeError
        String.fromCodePoint(3e-2); // RangeError
        String.fromCodePoint(NaN); // RangeError
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 140 error: ${e.message}`);
    }}

    // ---- fragment 141 ----
    try {{
        "foo".normalize("nfc"); // RangeError
        "foo".normalize(" NFC "); // RangeError
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 141 error: ${e.message}`);
    }}

    // ---- fragment 142 ----
    try {{
        "foo".normalize("NFC"); // 'foo'
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 142 error: ${e.message}`);
    }}

    // ---- fragment 143 ----
    try {{
        const invalid = new Date("nothing");
        invalid.toISOString(); // RangeError: invalid date
        invalid.toJSON(); // RangeError: invalid date
        JSON.stringify({ date: invalid }); // RangeError: invalid date
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 143 error: ${e.message}`);
    }}

    // ---- fragment 144 ----
    try {{
        invalid.toString(); // "Invalid Date"
        invalid.getDate(); // NaN
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 144 error: ${e.message}`);
    }}

    // ---- fragment 145 ----
    try {{
        new Date("05 October 2011 14:48 UTC").toISOString(); // "2011-10-05T14:48:00.000Z"
        new Date(1317826080).toISOString(); // "2011-10-05T14:48:00.000Z"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 145 error: ${e.message}`);
    }}

    // ---- fragment 146 ----
    try {{
        (77.1234).toExponential(-1); // RangeError
        (77.1234).toExponential(101); // RangeError

        (2.34).toFixed(-100); // RangeError
        (2.34).toFixed(1001); // RangeError

        (1234.5).toPrecision(-1); // RangeError
        (1234.5).toPrecision(101); // RangeError
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 146 error: ${e.message}`);
    }}

    // ---- fragment 147 ----
    try {{
        (77.1234).toExponential(4); // 7.7123e+1
        (77.1234).toExponential(2); // 7.71e+1

        (2.34).toFixed(1); // 2.3
        (2.35).toFixed(1); // 2.4 (note that it rounds up in this case)

        (5.123456).toPrecision(5); // 5.1235
        (5.123456).toPrecision(2); // 5.1
        (5.123456).toPrecision(1); // 5
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 147 error: ${e.message}`);
    }}

    // ---- fragment 148 ----
    try {{
        (42).toString(0);
        (42).toString(1);
        (42).toString(37);
        (42).toString(150);
        // You cannot use a string like this for formatting:
        (12071989).toString("MM-dd-yyyy");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 148 error: ${e.message}`);
    }}

    // ---- fragment 149 ----
    try {{
        (42).toString(2); // "101010" (binary)
        (13).toString(8); // "15" (octal)
        (0x42).toString(10); // "66" (decimal)
        (100000).toString(16); // "186a0" (hexadecimal)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 149 error: ${e.message}`);
    }}

    // ---- fragment 150 ----
    try {{
        "abc".repeat(Infinity); // RangeError
        "a".repeat(2 ** 30); // RangeError
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 150 error: ${e.message}`);
    }}

    // ---- fragment 151 ----
    try {{
        "abc".repeat(0); // ''
        "abc".repeat(1); // 'abc'
        "abc".repeat(2); // 'abcabc'
        "abc".repeat(3.5); // 'abcabcabc' (count will be converted to integer)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 151 error: ${e.message}`);
    }}

    // ---- fragment 152 ----
    try {{
        "abc".repeat(-1); // RangeError
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 152 error: ${e.message}`);
    }}

    // ---- fragment 153 ----
    try {{
        "abc".repeat(0); // ''
        "abc".repeat(1); // 'abc'
        "abc".repeat(2); // 'abcabc'
        "abc".repeat(3.5); // 'abcabcabc' (count will be converted to integer)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 153 error: ${e.message}`);
    }}

    // ---- fragment 154 ----
    try {{
        "use strict";

        const args = [1, 2, 3];
        console.log(Math.max(...args));

        function foo(...args) {
          console.log(args);
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 154 error: ${e.message}`);
    }}

    // ---- fragment 155 ----
    try {{
        0o3;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 155 error: ${e.message}`);
    }}

    // ---- fragment 156 ----
    try {{
        const colorEnum = { RED: 0, GREEN: 1, BLUE: 2 };
        const list = ["potatoes", "rice", "fries"];
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 156 error: ${e.message}`);
    }}

    // ---- fragment 157 ----
    try {{
        "use strict";
        class DocArchiver {}

        // SyntaxError: class is a reserved identifier
        // (throws in older browsers only, e.g. Firefox 44 and older)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 157 error: ${e.message}`);
    }}

    // ---- fragment 158 ----
    try {{
        const iterable = [10, 20, 30];

        for (let value of iterable) {
          value += 50;
          console.log(value);
        }
        // 60
        // 70
        // 80
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 158 error: ${e.message}`);
    }}

    // ---- fragment 159 ----
    try {{
        array.forEach((value) => {
          if (value === 5) {
            return;
          }
          // do something with value
        });
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 159 error: ${e.message}`);
    }}

    // ---- fragment 160 ----
    try {{
        for (const value of array) {
          if (value === 5) {
            continue;
          }
          // do something with value
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 160 error: ${e.message}`);
    }}

    // ---- fragment 161 ----
    try {{
        const obj = { a: 1, b: 2, c: 3 };

        for (const i in obj) {
          console.log(obj[i]);
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 161 error: ${e.message}`);
    }}

    // ---- fragment 162 ----
    try {{
        const arr = ["a", "b", "c"];

        for (let i = 2; i < arr.length; i++) {
          console.log(arr[i]);
        }

        // "c"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 162 error: ${e.message}`);
    }}

    // ---- fragment 163 ----
    try {{
        const life1 = "foo";
        const foo = life1;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 163 error: ${e.message}`);
    }}

    // ---- fragment 164 ----
    try {{
        // Wrap the number in parentheses
        alert(typeof (1).toString());

        // Add an extra dot for the number literal
        alert(typeof 2..toString());

        // Use square brackets
        alert(typeof 3["toString"]());
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 164 error: ${e.message}`);
    }}

    // ---- fragment 165 ----
    try {{
        "This is actually a string";
        42 - 13;
        const foo = "bar";
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 165 error: ${e.message}`);
    }}

    // ---- fragment 166 ----
    try {{
        /1{1}/u;
        /1{1,}/u;
        /1{1,2}/u;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 166 error: ${e.message}`);
    }}

    // ---- fragment 167 ----
    try {{
        /[\(\)\{\}]/v;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 167 error: ${e.message}`);
    }}

    // ---- fragment 168 ----
    try {{
        // If you want to match NULL followed by a digit, use a character class
        /[\0]0/u;
        // If you want to match a character by its character value, use \x
        /\x01/u;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 168 error: ${e.message}`);
    }}

    // ---- fragment 169 ----
    try {{
        // There's no need to escape the space
        /[\f\v\n\t ]/u;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 169 error: ${e.message}`);
    }}

    // ---- fragment 170 ----
    try {{
        /\p{Script=Latin}/u; // "Script=Latin" is a valid Unicode property
        /\p{Letter}/u; // "Letter" is valid value for General_Category
        /\p{RGI_Emoji_Flag_Sequence}/v; // Property of strings can only be used in "v" mode
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 170 error: ${e.message}`);
    }}

    // ---- fragment 171 ----
    try {{
        /[1-9]/; // Swap the range
        /[_\-=]/; // Escape the hyphen so it matches the literal character
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 171 error: ${e.message}`);
    }}

    // ---- fragment 172 ----
    try {{
        const re = new RegExp("pattern", "flags");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 172 error: ${e.message}`);
    }}

    // ---- fragment 173 ----
    try {{
        /foo/g;
        /foo/gims;
        /foo/uy;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 173 error: ${e.message}`);
    }}

    // ---- fragment 174 ----
    try {{
        const obj = {
          url: "/docs/Web",
        };
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 174 error: ${e.message}`);
    }}

    // ---- fragment 175 ----
    try {{
        /\u0065/u; // Lowercase "e"
        /\u{1f600}/u; // Grinning face emoji
        /\cA/u; // U+0001 (Start of Heading)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 175 error: ${e.message}`);
    }}

    // ---- fragment 176 ----
    try {{
        JSON.parse("[1, 2, 3, 4,]");
        JSON.parse('{"foo": 1,}');
        // SyntaxError JSON.parse: unexpected character
        // at line 1 column 14 of the JSON data
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 176 error: ${e.message}`);
    }}

    // ---- fragment 177 ----
    try {{
        JSON.parse("[1, 2, 3, 4]");
        JSON.parse('{"foo": 1}');
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 177 error: ${e.message}`);
    }}

    // ---- fragment 178 ----
    try {{
        JSON.parse("{'foo': 1}");
        // SyntaxError: JSON.parse: expected property name or '}'
        // at line 1 column 2 of the JSON data
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 178 error: ${e.message}`);
    }}

    // ---- fragment 179 ----
    try {{
        JSON.parse('{"foo": 1}');
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 179 error: ${e.message}`);
    }}

    // ---- fragment 180 ----
    try {{
        JSON.parse('{"foo": 01}');
        // SyntaxError: JSON.parse: expected ',' or '}' after property value
        // in object at line 1 column 2 of the JSON data

        JSON.parse('{"foo": 1.}');
        // SyntaxError: JSON.parse: unterminated fractional number
        // at line 1 column 2 of the JSON data
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 180 error: ${e.message}`);
    }}

    // ---- fragment 181 ----
    try {{
        JSON.parse('{"foo": 1}');
        JSON.parse('{"foo": 1.0}');
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 181 error: ${e.message}`);
    }}

    // ---- fragment 182 ----
    try {{
        start: {
          console.log("Hello, world!");
          if (Math.random() > 0.5) {
            break start;
          }
          console.log("Maybe I'm logged");
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 182 error: ${e.message}`);
    }}

    // ---- fragment 183 ----
    try {{
        console.log("PI: " + Math.PI);
        // "PI: 3.141592653589793"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 183 error: ${e.message}`);
    }}

    // ---- fragment 184 ----
    try {{
        console.log(`PI: ${Math.PI}`);
        console.log("PI:", Math.PI);
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 184 error: ${e.message}`);
    }}

    // ---- fragment 185 ----
    try {{
        console.log('"Java" + "Script" = "' + "Java" + 'Script"');
        // '"Java" + "Script" = "JavaScript"'
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 185 error: ${e.message}`);
    }}

    // ---- fragment 186 ----
    try {{
        if (condition) {
          // do something if the condition is true
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 186 error: ${e.message}`);
    }}

    // ---- fragment 187 ----
    try {{
        if (Math.PI < 3) {
          console.log("wait what?");
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 187 error: ${e.message}`);
    }}

    // ---- fragment 188 ----
    try {{
        if (done === true) {
          console.log("we are done!");
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 188 error: ${e.message}`);
    }}

    // ---- fragment 189 ----
    try {{
        if (done) {
          console.log("we are done!");
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 189 error: ${e.message}`);
    }}

    // ---- fragment 190 ----
    try {{
        const list = [1, 2];

        const instruments = ["Ukulele", "Guitar", "Piano"];

        const data = [{ foo: "bar" }, { bar: "foo" }];
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 190 error: ${e.message}`);
    }}

    // ---- fragment 191 ----
    try {{
        function charge() {
          if (sunny) {
            useSolarCells();
          } else {
            promptBikeRide();
          }
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 191 error: ${e.message}`);
    }}

    // ---- fragment 192 ----
    try {{
        (function () {
          if (Math.random() < 0.01) {
            doSomething();
          }
        })();
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 192 error: ${e.message}`);
    }}

    // ---- fragment 193 ----
    try {{
        const obj = {
          a: 1,
          b: { myProp: 2 },
          c: 3,
        };
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 193 error: ${e.message}`);
    }}

    // ---- fragment 194 ----
    try {{
        const COLUMNS = 80;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 194 error: ${e.message}`);
    }}

    // ---- fragment 195 ----
    try {{
        let columns;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 195 error: ${e.message}`);
    }}

    // ---- fragment 196 ----
    try {{
        function square(number) {
          return number * number;
        }

        function greet(greeting) {
          return greeting;
        }

        function log(arg) {
          console.log(arg);
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 196 error: ${e.message}`);
    }}

    // ---- fragment 197 ----
    try {{
        square(2); // 4

        greet("Howdy"); // "Howdy"

        log({ obj: "value" }); // { obj: "value" }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 197 error: ${e.message}`);
    }}

    // ---- fragment 198 ----
    try {{
        obj.foo.bar; // "baz"
        // or alternatively
        obj["foo"]["bar"]; // "baz"

        // computed properties require square brackets
        obj.foo["bar" + i]; // "baz2"
        // or as template literal
        obj.foo[`bar${i}`]; // "baz2"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 198 error: ${e.message}`);
    }}

    // ---- fragment 199 ----
    try {{
        console.log("Hello" + "World");
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 199 error: ${e.message}`);
    }}

    // ---- fragment 200 ----
    try {{
        // Matches two characters that are not an emoji flag sequence
        /(?!\p{RGI_Emoji_Flag_Sequence})../v;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 200 error: ${e.message}`);
    }}

    // ---- fragment 201 ----
    try {{
        /b+/; // b is a character, it can be repeated
        /(\*hello\*)/; // Escape the asterisks to match them literally
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 201 error: ${e.message}`);
    }}

    // ---- fragment 202 ----
    try {{
        /1{1,2}/;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 202 error: ${e.message}`);
    }}

    // ---- fragment 203 ----
    try {{
        "\xA9";
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 203 error: ${e.message}`);
    }}

    // ---- fragment 204 ----
    try {{
        String.raw`\251`; // A string containing four characters
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 204 error: ${e.message}`);
    }}

    // ---- fragment 205 ----
    try {{
        function replacer(match, ...args) {
          const offset = args.at(-2);
          const string = args.at(-1);
        }

        function doSomething(arg1, arg2, ...otherArgs) {}
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 205 error: ${e.message}`);
    }}

    // ---- fragment 206 ----
    try {{
        // Only setting the prototype once
        const obj = { __proto__: { a: 1 } };

        // These syntaxes all create a property called "__proto__" and can coexist
        // They would overwrite each other and the last one is actually used
        const __proto__ = null;
        const obj2 = {
          ["__proto__"]: {},
          __proto__,
          __proto__() {},
          get __proto__() {
            return 1;
          },
        };
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 206 error: ${e.message}`);
    }}

    // ---- fragment 207 ----
    try {{
        // All { and } need to be escaped
        /\{\{MDN_Macro\}\}/u;
        // The ] needs to be escaped
        /\[sic\]/u;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 207 error: ${e.message}`);
    }}

    // ---- fragment 208 ----
    try {{
        function f(arg) {
          arg = "foo";
        }

        function g(arg) {
          let bar = "foo";
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 208 error: ${e.message}`);
    }}

    // ---- fragment 209 ----
    try {{
        function doSomething(...args) {
          // args is always an array
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 209 error: ${e.message}`);
    }}

    // ---- fragment 210 ----
    try {{
        function cheer(score) {
          if (score === 147) {
            return "Maximum!";
          }
          if (score > 100) {
            return "Century!";
          }
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 210 error: ${e.message}`);
    }}

    // ---- fragment 211 ----
    try {{
        (-a) ** b
        -(a ** b)
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 211 error: ${e.message}`);
    }}

    // ---- fragment 212 ----
    try {{
        function taylorSin(x) {
          return (n) => ((-1) ** n * x ** (2 * n + 1)) / factorial(2 * n + 1);
        }
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 212 error: ${e.message}`);
    }}

    // ---- fragment 213 ----
    try {{
        Warning: SyntaxError: Using //@ to indicate sourceURL pragmas is deprecated. Use //# instead

        Warning: SyntaxError: Using //@ to indicate sourceMappingURL pragmas is deprecated. Use //# instead
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 213 error: ${e.message}`);
    }}

    // ---- fragment 214 ----
    try {{
        Object.defineProperty({}, "key", 1);
        // TypeError: 1 is not a non-null object

        Object.defineProperty({}, "key", null);
        // TypeError: null is not a non-null object
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 214 error: ${e.message}`);
    }}

    // ---- fragment 215 ----
    try {{
        Object.defineProperty({}, "key", { value: "foo", writable: false });
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 215 error: ${e.message}`);
    }}

    // ---- fragment 216 ----
    try {{
        Object.setPrototypeOf(Object.prototype, {});
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 216 error: ${e.message}`);
    }}

    // ---- fragment 217 ----
    try {{
        const obj = {};
        Object.preventExtensions(obj);
        Object.setPrototypeOf(obj, {});
        // TypeError: can't set prototype of this object
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 217 error: ${e.message}`);
    }}

    // ---- fragment 218 ----
    try {{
        const circularReference = { otherData: 123 };
        circularReference.myself = circularReference;
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 218 error: ${e.message}`);
    }}

    // ---- fragment 219 ----
    try {{
        JSON.stringify(circularReference);
        // TypeError: cyclic object value
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 219 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 220 error: ${e.message}`);
    }}

    // ---- fragment 221 ----
    try {{
        "abc".matchAll(/./); // TypeError
        "abc".replaceAll(/./, "f"); // TypeError
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 221 error: ${e.message}`);
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
        console.error(`[testBuiltins] fragment 222 error: ${e.message}`);
    }}

    // ---- fragment 223 ----
    try {{
        "abc".match(/./); // [ "a" ]
        "abc".replace(/./, "f"); // "fbc"

        [..././[Symbol.matchAll]("abc")]; // [[ "a" ]]
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 223 error: ${e.message}`);
    }}

    // ---- fragment 224 ----
    try {{
        null.foo;
        // TypeError: null has no properties

        undefined.bar;
        // TypeError: undefined has no properties
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 224 error: ${e.message}`);
    }}

    // ---- fragment 225 ----
    try {{
        encodeURI("\uD800");
        // "URIError: malformed URI sequence"

        encodeURI("\uDFFF");
        // "URIError: malformed URI sequence"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 225 error: ${e.message}`);
    }}

    // ---- fragment 226 ----
    try {{
        encodeURI("\uD800\uDFFF");
        // "%F0%90%8F%BF"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 226 error: ${e.message}`);
    }}

    // ---- fragment 227 ----
    try {{
        decodeURIComponent("%E0%A4%A");
        // "URIError: malformed URI sequence"
    }} catch (e) {{
        console.error(`[testBuiltins] fragment 227 error: ${e.message}`);
    }}

}
module.exports = { testBuiltins };