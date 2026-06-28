// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 168
// Generated: 2026-06-28

function testExpressions() {
    // ---- fragment 0 ----
    try {{
        const output = void 1;
        console.log(output);

        void console.log("expression evaluated");

        void (function iife() {
          console.log("iife is executed");
        })();

        void function test() {
          console.log("test function executed");
        };
        try {
          test();
        } catch (e) {
          console.log("test function is not defined");
        }
    }} catch (e) {{
        console.error(`[testExpressions] fragment 0 error: ${e.message}`);
    }}

    // ---- fragment 1 ----
    try {{
        void expression
    }} catch (e) {{
        console.error(`[testExpressions] fragment 1 error: ${e.message}`);
    }}

    // ---- fragment 2 ----
    try {{
        void 2 === "2"; // (void 2) === '2', returns false
        void (2 === "2"); // void (2 === '2'), returns undefined
    }} catch (e) {{
        console.error(`[testExpressions] fragment 2 error: ${e.message}`);
    }}

    // ---- fragment 3 ----
    try {{
        void function () {
          console.log("Executed!");
        }();

        // Logs "Executed!"
    }} catch (e) {{
        console.error(`[testExpressions] fragment 3 error: ${e.message}`);
    }}

    // ---- fragment 4 ----
    try {{
        (function () {
          console.log("Executed!");
        })();
    }} catch (e) {{
        console.error(`[testExpressions] fragment 4 error: ${e.message}`);
    }}

    // ---- fragment 5 ----
    try {{
        checkbox.onclick = () => doSomething();
    }} catch (e) {{
        console.error(`[testExpressions] fragment 5 error: ${e.message}`);
    }}

    // ---- fragment 6 ----
    try {{
        checkbox.onclick = () => void doSomething();
    }} catch (e) {{
        console.error(`[testExpressions] fragment 6 error: ${e.message}`);
    }}

    // ---- fragment 7 ----
    try {{
        const a = 5; // 00000000000000000000000000000101
        const b = -3; // 11111111111111111111111111111101

        console.log(~a); // 11111111111111111111111111111010

        console.log(~b); // 00000000000000000000000000000010
    }} catch (e) {{
        console.error(`[testExpressions] fragment 7 error: ${e.message}`);
    }}

    // ---- fragment 8 ----
    try {{
        ~x
    }} catch (e) {{
        console.error(`[testExpressions] fragment 8 error: ${e.message}`);
    }}

    // ---- fragment 9 ----
    try {{
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[testExpressions] fragment 9 error: ${e.message}`);
    }}

    // ---- fragment 10 ----
    try {{
        ~0; // -1
        ~-1; // 0
        ~1; // -2

        ~0n; // -1n
        ~4294967295n; // -4294967296n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 10 error: ${e.message}`);
    }}

    // ---- fragment 11 ----
    try {{
        const a = 3;
        const b = -2;

        console.log(!(a > 0 || b > 0));
    }} catch (e) {{
        console.error(`[testExpressions] fragment 11 error: ${e.message}`);
    }}

    // ---- fragment 12 ----
    try {{
        !x
    }} catch (e) {{
        console.error(`[testExpressions] fragment 12 error: ${e.message}`);
    }}

    // ---- fragment 13 ----
    try {{
        !true; // !t returns false
        !false; // !f returns true
        !""; // !f returns true
        !"Cat"; // !t returns false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 13 error: ${e.message}`);
    }}

    // ---- fragment 14 ----
    try {{
        !!bCondition
    }} catch (e) {{
        console.error(`[testExpressions] fragment 14 error: ${e.message}`);
    }}

    // ---- fragment 15 ----
    try {{
        bCondition
    }} catch (e) {{
        console.error(`[testExpressions] fragment 15 error: ${e.message}`);
    }}

    // ---- fragment 16 ----
    try {{
        console.log(3 ** 4);

        console.log(10 ** -2);

        console.log(2 ** (3 ** 2));

        console.log((2 ** 3) ** 2);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 16 error: ${e.message}`);
    }}

    // ---- fragment 17 ----
    try {{
        x ** y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 17 error: ${e.message}`);
    }}

    // ---- fragment 18 ----
    try {{
        2 ** 3; // 8
        3 ** 2; // 9
        3 ** 2.5; // 15.588457268119896
        10 ** -1; // 0.1
        2 ** 1024; // Infinity
        NaN ** 2; // NaN
        NaN ** 0; // 1
        1 ** Infinity; // NaN
    }} catch (e) {{
        console.error(`[testExpressions] fragment 18 error: ${e.message}`);
    }}

    // ---- fragment 19 ----
    try {{
        2 ** "3"; // 8
        2 ** "hello"; // NaN
    }} catch (e) {{
        console.error(`[testExpressions] fragment 19 error: ${e.message}`);
    }}

    // ---- fragment 20 ----
    try {{
        2n ** 3n; // 8n
        2n ** 1024n; // A very large number, but not Infinity
    }} catch (e) {{
        console.error(`[testExpressions] fragment 20 error: ${e.message}`);
    }}

    // ---- fragment 21 ----
    try {{
        2n ** 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 ** 2n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[testExpressions] fragment 21 error: ${e.message}`);
    }}

    // ---- fragment 22 ----
    try {{
        2n ** BigInt(2); // 4n
        Number(2n) ** 2; // 4
    }} catch (e) {{
        console.error(`[testExpressions] fragment 22 error: ${e.message}`);
    }}

    // ---- fragment 23 ----
    try {{
        2 ** 3 ** 2; // 512
        2 ** (3 ** 2); // 512
        (2 ** 3) ** 2; // 64
    }} catch (e) {{
        console.error(`[testExpressions] fragment 23 error: ${e.message}`);
    }}

    // ---- fragment 24 ----
    try {{
        -(2 ** 2); // -4
    }} catch (e) {{
        console.error(`[testExpressions] fragment 24 error: ${e.message}`);
    }}

    // ---- fragment 25 ----
    try {{
        (-2) ** 2; // 4
    }} catch (e) {{
        console.error(`[testExpressions] fragment 25 error: ${e.message}`);
    }}

    // ---- fragment 26 ----
    try {{
        console.log(3 * 4);

        console.log(-3 * 4);

        console.log("3" * 2);

        console.log("foo" * 2);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 26 error: ${e.message}`);
    }}

    // ---- fragment 27 ----
    try {{
        x * y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 27 error: ${e.message}`);
    }}

    // ---- fragment 28 ----
    try {{
        2 * 2; // 4
        -2 * 2; // -4

        Infinity * 0; // NaN
        Infinity * Infinity; // Infinity
    }} catch (e) {{
        console.error(`[testExpressions] fragment 28 error: ${e.message}`);
    }}

    // ---- fragment 29 ----
    try {{
        "foo" * 2; // NaN
        "2" * 2; // 4
    }} catch (e) {{
        console.error(`[testExpressions] fragment 29 error: ${e.message}`);
    }}

    // ---- fragment 30 ----
    try {{
        2n * 2n; // 4n
        -2n * 2n; // -4n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 30 error: ${e.message}`);
    }}

    // ---- fragment 31 ----
    try {{
        2n * 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 * 2n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[testExpressions] fragment 31 error: ${e.message}`);
    }}

    // ---- fragment 32 ----
    try {{
        2n * BigInt(2); // 4n
        Number(2n) * 2; // 4
    }} catch (e) {{
        console.error(`[testExpressions] fragment 32 error: ${e.message}`);
    }}

    // ---- fragment 33 ----
    try {{
        console.log(12 / 2);

        console.log(3 / 2);

        console.log(6 / "3");

        console.log(2 / 0);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 33 error: ${e.message}`);
    }}

    // ---- fragment 34 ----
    try {{
        x / y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 34 error: ${e.message}`);
    }}

    // ---- fragment 35 ----
    try {{
        1 / 2; // 0.5
        Math.floor(3 / 2); // 1
        1.0 / 2.0; // 0.5

        2 / 0; // Infinity
        2.0 / 0.0; // Infinity, because 0.0 === 0
        2.0 / -0.0; // -Infinity
    }} catch (e) {{
        console.error(`[testExpressions] fragment 35 error: ${e.message}`);
    }}

    // ---- fragment 36 ----
    try {{
        5 / "2"; // 2.5
        5 / "foo"; // NaN
    }} catch (e) {{
        console.error(`[testExpressions] fragment 36 error: ${e.message}`);
    }}

    // ---- fragment 37 ----
    try {{
        1n / 2n; // 0n
        5n / 3n; // 1n
        -1n / 3n; // 0n
        1n / -3n; // 0n

        2n / 0n; // RangeError: BigInt division by zero
    }} catch (e) {{
        console.error(`[testExpressions] fragment 37 error: ${e.message}`);
    }}

    // ---- fragment 38 ----
    try {{
        2n / 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 / 2n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[testExpressions] fragment 38 error: ${e.message}`);
    }}

    // ---- fragment 39 ----
    try {{
        2n / BigInt(2); // 1n
        Number(2n) / 2; // 1
    }} catch (e) {{
        console.error(`[testExpressions] fragment 39 error: ${e.message}`);
    }}

    // ---- fragment 40 ----
    try {{
        console.log(13 % 5);

        console.log(-13 % 5);

        console.log(4 % 2);

        console.log(-4 % 2);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 40 error: ${e.message}`);
    }}

    // ---- fragment 41 ----
    try {{
        x % y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 41 error: ${e.message}`);
    }}

    // ---- fragment 42 ----
    try {{
        13 % 5; // 3
        1 % -2; // 1
        1 % 2; // 1
        2 % 3; // 2
        5.5 % 2; // 1.5

        3n % 2n; // 1n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 42 error: ${e.message}`);
    }}

    // ---- fragment 43 ----
    try {{
        -13 % 5; // -3
        -1 % 2; // -1
        -4 % 2; // -0

        -3n % 2n; // -1n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 43 error: ${e.message}`);
    }}

    // ---- fragment 44 ----
    try {{
        NaN % 2; // NaN
    }} catch (e) {{
        console.error(`[testExpressions] fragment 44 error: ${e.message}`);
    }}

    // ---- fragment 45 ----
    try {{
        Infinity % 2; // NaN
        Infinity % 0; // NaN
        Infinity % Infinity; // NaN
        2 % Infinity; // 2
        0 % Infinity; // 0
    }} catch (e) {{
        console.error(`[testExpressions] fragment 45 error: ${e.message}`);
    }}

    // ---- fragment 46 ----
    try {{
        console.log(2 + 2);

        console.log(2 + true);

        console.log("hello " + "everyone");

        console.log(2001 + ": A Space Odyssey");
    }} catch (e) {{
        console.error(`[testExpressions] fragment 46 error: ${e.message}`);
    }}

    // ---- fragment 47 ----
    try {{
        x + y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 47 error: ${e.message}`);
    }}

    // ---- fragment 48 ----
    try {{
        const t = Temporal.Now.instant();
        "" + t; // Throws TypeError
        `${t}`; // '2022-07-31T04:48:56.113918308Z'
        "".concat(t); // '2022-07-31T04:48:56.113918308Z'
    }} catch (e) {{
        console.error(`[testExpressions] fragment 48 error: ${e.message}`);
    }}

    // ---- fragment 49 ----
    try {{
        1 + 2; // 3
    }} catch (e) {{
        console.error(`[testExpressions] fragment 49 error: ${e.message}`);
    }}

    // ---- fragment 50 ----
    try {{
        true + 1; // 2
        false + false; // 0
    }} catch (e) {{
        console.error(`[testExpressions] fragment 50 error: ${e.message}`);
    }}

    // ---- fragment 51 ----
    try {{
        1n + 2n; // 3n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 51 error: ${e.message}`);
    }}

    // ---- fragment 52 ----
    try {{
        1n + 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 + 1n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[testExpressions] fragment 52 error: ${e.message}`);
    }}

    // ---- fragment 53 ----
    try {{
        "1" + 2n; // "12"
    }} catch (e) {{
        console.error(`[testExpressions] fragment 53 error: ${e.message}`);
    }}

    // ---- fragment 54 ----
    try {{
        1n + BigInt(2); // 3n
        Number(1n) + 2; // 3
    }} catch (e) {{
        console.error(`[testExpressions] fragment 54 error: ${e.message}`);
    }}

    // ---- fragment 55 ----
    try {{
        "foo" + "bar"; // "foobar"
        5 + "foo"; // "5foo"
        "foo" + false; // "foofalse"
        "2" + 2; // "22"
    }} catch (e) {{
        console.error(`[testExpressions] fragment 55 error: ${e.message}`);
    }}

    // ---- fragment 56 ----
    try {{
        console.log(5 - 3);

        console.log(3.5 - 5);

        console.log(5 - "hello");

        console.log(5 - true);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 56 error: ${e.message}`);
    }}

    // ---- fragment 57 ----
    try {{
        x - y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 57 error: ${e.message}`);
    }}

    // ---- fragment 58 ----
    try {{
        5 - 3; // 2
        3 - 5; // -2
    }} catch (e) {{
        console.error(`[testExpressions] fragment 58 error: ${e.message}`);
    }}

    // ---- fragment 59 ----
    try {{
        "foo" - 3; // NaN; "foo" is converted to the number NaN
        5 - "3"; // 2; "3" is converted to the number 3
    }} catch (e) {{
        console.error(`[testExpressions] fragment 59 error: ${e.message}`);
    }}

    // ---- fragment 60 ----
    try {{
        2n - 1n; // 1n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 60 error: ${e.message}`);
    }}

    // ---- fragment 61 ----
    try {{
        2n - 1; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 - 1n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[testExpressions] fragment 61 error: ${e.message}`);
    }}

    // ---- fragment 62 ----
    try {{
        2n - BigInt(1); // 1n
        Number(2n) - 1; // 1
    }} catch (e) {{
        console.error(`[testExpressions] fragment 62 error: ${e.message}`);
    }}

    // ---- fragment 63 ----
    try {{
        console.log(5 > 3);

        console.log(3 > 3);

        // Compare bigint to number
        console.log(3n > 5);

        console.log("ab" > "aa");
    }} catch (e) {{
        console.error(`[testExpressions] fragment 63 error: ${e.message}`);
    }}

    // ---- fragment 64 ----
    try {{
        x > y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 64 error: ${e.message}`);
    }}

    // ---- fragment 65 ----
    try {{
        "a" > "b"; // false
        "a" > "a"; // false
        "a" > "3"; // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 65 error: ${e.message}`);
    }}

    // ---- fragment 66 ----
    try {{
        "5" > 3; // true
        "3" > 3; // false
        "3" > 5; // false

        "hello" > 5; // false
        5 > "hello"; // false

        "5" > 3n; // true
        "3" > 5n; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 66 error: ${e.message}`);
    }}

    // ---- fragment 67 ----
    try {{
        5 > 3; // true
        3 > 3; // false
        3 > 5; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 67 error: ${e.message}`);
    }}

    // ---- fragment 68 ----
    try {{
        5n > 3; // true
        3 > 5n; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 68 error: ${e.message}`);
    }}

    // ---- fragment 69 ----
    try {{
        true > false; // true
        false > true; // false

        true > 0; // true
        true > 1; // false

        null > 0; // false
        1 > null; // true

        undefined > 3; // false
        3 > undefined; // false

        3 > NaN; // false
        NaN > 3; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 69 error: ${e.message}`);
    }}

    // ---- fragment 70 ----
    try {{
        console.log(5 <= 3);

        console.log(3 <= 3);

        // Compare bigint to number
        console.log(3n <= 5);

        console.log("aa" <= "ab");
    }} catch (e) {{
        console.error(`[testExpressions] fragment 70 error: ${e.message}`);
    }}

    // ---- fragment 71 ----
    try {{
        x <= y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 71 error: ${e.message}`);
    }}

    // ---- fragment 72 ----
    try {{
        "a" <= "b"; // true
        "a" <= "a"; // true
        "a" <= "3"; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 72 error: ${e.message}`);
    }}

    // ---- fragment 73 ----
    try {{
        "5" <= 3; // false
        "3" <= 3; // true
        "3" <= 5; // true

        "hello" <= 5; // false
        5 <= "hello"; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 73 error: ${e.message}`);
    }}

    // ---- fragment 74 ----
    try {{
        5 <= 3; // false
        3 <= 3; // true
        3 <= 5; // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 74 error: ${e.message}`);
    }}

    // ---- fragment 75 ----
    try {{
        5n <= 3; // false
        3 <= 3n; // true
        3 <= 5n; // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 75 error: ${e.message}`);
    }}

    // ---- fragment 76 ----
    try {{
        true <= false; // false
        true <= true; // true
        false <= true; // true

        true <= 0; // false
        true <= 1; // true

        null <= 0; // true
        1 <= null; // false

        undefined <= 3; // false
        3 <= undefined; // false

        3 <= NaN; // false
        NaN <= 3; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 76 error: ${e.message}`);
    }}

    // ---- fragment 77 ----
    try {{
        console.log(5 >= 3);

        console.log(3 >= 3);

        // Compare bigint to number
        console.log(3n >= 5);

        console.log("ab" >= "aa");
    }} catch (e) {{
        console.error(`[testExpressions] fragment 77 error: ${e.message}`);
    }}

    // ---- fragment 78 ----
    try {{
        x >= y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 78 error: ${e.message}`);
    }}

    // ---- fragment 79 ----
    try {{
        "a" >= "b"; // false
        "a" >= "a"; // true
        "a" >= "3"; // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 79 error: ${e.message}`);
    }}

    // ---- fragment 80 ----
    try {{
        "5" >= 3; // true
        "3" >= 3; // true
        "3" >= 5; // false

        "hello" >= 5; // false
        5 >= "hello"; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 80 error: ${e.message}`);
    }}

    // ---- fragment 81 ----
    try {{
        5 >= 3; // true
        3 >= 3; // true
        3 >= 5; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 81 error: ${e.message}`);
    }}

    // ---- fragment 82 ----
    try {{
        5n >= 3; // true
        3 >= 3n; // true
        3 >= 5n; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 82 error: ${e.message}`);
    }}

    // ---- fragment 83 ----
    try {{
        true >= false; // true
        true >= true; // true
        false >= true; // false

        true >= 0; // true
        true >= 1; // true

        null >= 0; // true
        1 >= null; // true

        undefined >= 3; // false
        3 >= undefined; // false

        3 >= NaN; // false
        NaN >= 3; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 83 error: ${e.message}`);
    }}

    // ---- fragment 84 ----
    try {{
        console.log(1 != 1);

        console.log("hello" != "hello");

        console.log("1" != 1);

        console.log(0 != false);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 84 error: ${e.message}`);
    }}

    // ---- fragment 85 ----
    try {{
        x != y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 85 error: ${e.message}`);
    }}

    // ---- fragment 86 ----
    try {{
        x != y;

        !(x == y);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 86 error: ${e.message}`);
    }}

    // ---- fragment 87 ----
    try {{
        3 != "3"; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 87 error: ${e.message}`);
    }}

    // ---- fragment 88 ----
    try {{
        3 !== "3"; // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 88 error: ${e.message}`);
    }}

    // ---- fragment 89 ----
    try {{
        1 != 2; // true
        "hello" != "hola"; // true

        1 != 1; // false
        "hello" != "hello"; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 89 error: ${e.message}`);
    }}

    // ---- fragment 90 ----
    try {{
        "1" != 1; // false
        1 != "1"; // false
        0 != false; // false
        0 != null; // true
        0 != undefined; // true
        0 != !!null; // false, look at Logical NOT operator
        0 != !!undefined; // false, look at Logical NOT operator
        null != undefined; // false

        const number1 = new Number(3);
        const number2 = new Number(3);
        number1 != 3; // false
        number1 != number2; // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 90 error: ${e.message}`);
    }}

    // ---- fragment 91 ----
    try {{
        const object1 = {
          key: "value",
        };

        const object2 = {
          key: "value",
        };

        console.log(object1 != object2); // true
        console.log(object1 != object1); // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 91 error: ${e.message}`);
    }}

    // ---- fragment 92 ----
    try {{
        console.log(1 === 1);

        console.log("hello" === "hello");

        console.log("1" === 1);

        console.log(0 === false);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 92 error: ${e.message}`);
    }}

    // ---- fragment 93 ----
    try {{
        x === y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 93 error: ${e.message}`);
    }}

    // ---- fragment 94 ----
    try {{
        "hello" === "hello"; // true
        "hello" === "hola"; // false

        3 === 3; // true
        3 === 4; // false

        true === true; // true
        true === false; // false

        null === null; // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 94 error: ${e.message}`);
    }}

    // ---- fragment 95 ----
    try {{
        "3" === 3; // false
        true === 1; // false
        null === undefined; // false
        3 === new Number(3); // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 95 error: ${e.message}`);
    }}

    // ---- fragment 96 ----
    try {{
        const object1 = {
          key: "value",
        };

        const object2 = {
          key: "value",
        };

        console.log(object1 === object2); // false
        console.log(object1 === object1); // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 96 error: ${e.message}`);
    }}

    // ---- fragment 97 ----
    try {{
        console.log(1 !== 1);

        console.log("hello" !== "hello");

        console.log("1" !== 1);

        console.log(0 !== false);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 97 error: ${e.message}`);
    }}

    // ---- fragment 98 ----
    try {{
        x !== y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 98 error: ${e.message}`);
    }}

    // ---- fragment 99 ----
    try {{
        x !== y;

        !(x === y);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 99 error: ${e.message}`);
    }}

    // ---- fragment 100 ----
    try {{
        3 !== "3"; // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 100 error: ${e.message}`);
    }}

    // ---- fragment 101 ----
    try {{
        "hello" !== "hello"; // false
        "hello" !== "hola"; // true

        3 !== 3; // false
        3 !== 4; // true

        true !== true; // false
        true !== false; // true

        null !== null; // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 101 error: ${e.message}`);
    }}

    // ---- fragment 102 ----
    try {{
        "3" !== 3; // true
        true !== 1; // true
        null !== undefined; // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 102 error: ${e.message}`);
    }}

    // ---- fragment 103 ----
    try {{
        const object1 = {
          key: "value",
        };

        const object2 = {
          key: "value",
        };

        console.log(object1 !== object2); // true
        console.log(object1 !== object1); // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 103 error: ${e.message}`);
    }}

    // ---- fragment 104 ----
    try {{
        const a = 5; // 00000000000000000000000000000101
        const b = 2; // 00000000000000000000000000000010

        console.log(a << b); // 00000000000000000000000000010100
    }} catch (e) {{
        console.error(`[testExpressions] fragment 104 error: ${e.message}`);
    }}

    // ---- fragment 105 ----
    try {{
        x << y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 105 error: ${e.message}`);
    }}

    // ---- fragment 106 ----
    try {{
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[testExpressions] fragment 106 error: ${e.message}`);
    }}

    // ---- fragment 107 ----
    try {{
        9 << 3; // 72

        // 9 * (2 ** 3) = 9 * (8) = 72

        9n << 3n; // 72n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 107 error: ${e.message}`);
    }}

    // ---- fragment 108 ----
    try {{
        const a = 5; //  00000000000000000000000000000101
        const b = 2; //  00000000000000000000000000000010
        const c = -5; //  11111111111111111111111111111011

        console.log(a >> b); //  00000000000000000000000000000001

        console.log(c >> b); //  11111111111111111111111111111110
    }} catch (e) {{
        console.error(`[testExpressions] fragment 108 error: ${e.message}`);
    }}

    // ---- fragment 109 ----
    try {{
        x >> y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 109 error: ${e.message}`);
    }}

    // ---- fragment 110 ----
    try {{
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[testExpressions] fragment 110 error: ${e.message}`);
    }}

    // ---- fragment 111 ----
    try {{
        9 >> 2; // 2
        -9 >> 2; // -3

        9n >> 2n; // 2n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 111 error: ${e.message}`);
    }}

    // ---- fragment 112 ----
    try {{
        const a = 5; //  00000000000000000000000000000101
        const b = 2; //  00000000000000000000000000000010
        const c = -5; //  11111111111111111111111111111011

        console.log(a >>> b); //  00000000000000000000000000000001

        console.log(c >>> b); //  00111111111111111111111111111110
    }} catch (e) {{
        console.error(`[testExpressions] fragment 112 error: ${e.message}`);
    }}

    // ---- fragment 113 ----
    try {{
        x >>> y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 113 error: ${e.message}`);
    }}

    // ---- fragment 114 ----
    try {{
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[testExpressions] fragment 114 error: ${e.message}`);
    }}

    // ---- fragment 115 ----
    try {{
        9 >>> 2; // 2
        -9 >>> 2; // 1073741821
    }} catch (e) {{
        console.error(`[testExpressions] fragment 115 error: ${e.message}`);
    }}

    // ---- fragment 116 ----
    try {{
        9n >>> 2n; // TypeError: BigInts have no unsigned right shift, use >> instead
    }} catch (e) {{
        console.error(`[testExpressions] fragment 116 error: ${e.message}`);
    }}

    // ---- fragment 117 ----
    try {{
        const a = 5; // 00000000000000000000000000000101
        const b = 3; // 00000000000000000000000000000011

        console.log(a & b); // 00000000000000000000000000000001
    }} catch (e) {{
        console.error(`[testExpressions] fragment 117 error: ${e.message}`);
    }}

    // ---- fragment 118 ----
    try {{
        x & y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 118 error: ${e.message}`);
    }}

    // ---- fragment 119 ----
    try {{
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[testExpressions] fragment 119 error: ${e.message}`);
    }}

    // ---- fragment 120 ----
    try {{
        // 9  (00000000000000000000000000001001)
        // 14 (00000000000000000000000000001110)

        14 & 9;
        // 8  (00000000000000000000000000001000)

        14n & 9n; // 8n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 120 error: ${e.message}`);
    }}

    // ---- fragment 121 ----
    try {{
        const a = 5; // 00000000000000000000000000000101
        const b = 3; // 00000000000000000000000000000011

        console.log(a | b); // 00000000000000000000000000000111
    }} catch (e) {{
        console.error(`[testExpressions] fragment 121 error: ${e.message}`);
    }}

    // ---- fragment 122 ----
    try {{
        x | y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 122 error: ${e.message}`);
    }}

    // ---- fragment 123 ----
    try {{
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[testExpressions] fragment 123 error: ${e.message}`);
    }}

    // ---- fragment 124 ----
    try {{
        // 9  (00000000000000000000000000001001)
        // 14 (00000000000000000000000000001110)

        14 | 9;
        // 15 (00000000000000000000000000001111)

        14n | 9n; // 15n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 124 error: ${e.message}`);
    }}

    // ---- fragment 125 ----
    try {{
        const a = 5; // 00000000000000000000000000000101
        const b = 3; // 00000000000000000000000000000011

        console.log(a ^ b); // 00000000000000000000000000000110
    }} catch (e) {{
        console.error(`[testExpressions] fragment 125 error: ${e.message}`);
    }}

    // ---- fragment 126 ----
    try {{
        x ^ y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 126 error: ${e.message}`);
    }}

    // ---- fragment 127 ----
    try {{
        Before: 11100110111110100000000000000110000000000001
        After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[testExpressions] fragment 127 error: ${e.message}`);
    }}

    // ---- fragment 128 ----
    try {{
        // 9  (00000000000000000000000000001001)
        // 14 (00000000000000000000000000001110)

        14 ^ 9;
        // 7  (00000000000000000000000000000111)

        14n ^ 9n; // 7n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 128 error: ${e.message}`);
    }}

    // ---- fragment 129 ----
    try {{
        const a = 3;
        const b = -2;

        console.log(a > 0 && b > 0);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 129 error: ${e.message}`);
    }}

    // ---- fragment 130 ----
    try {{
        x && y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 130 error: ${e.message}`);
    }}

    // ---- fragment 131 ----
    try {{
        result = "" && "foo"; // result is assigned "" (empty string)
        result = 2 && 0; // result is assigned 0
        result = "foo" && 4; // result is assigned 4
    }} catch (e) {{
        console.error(`[testExpressions] fragment 131 error: ${e.message}`);
    }}

    // ---- fragment 132 ----
    try {{
        function A() {
          console.log("called A");
          return false;
        }
        function B() {
          console.log("called B");
          return true;
        }

        console.log(A() && B());
        // Logs "called A" to the console due to the call for function A,
        // && evaluates to false (function A returns false), then false is logged to the console;
        // the AND operator short-circuits here and ignores function B
    }} catch (e) {{
        console.error(`[testExpressions] fragment 132 error: ${e.message}`);
    }}

    // ---- fragment 133 ----
    try {{
        true || false && false; // true
        true && (false || false); // false
        (2 === 3) || (4 < 0) && (1 === 1); // false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 133 error: ${e.message}`);
    }}

    // ---- fragment 134 ----
    try {{
        a1 = true && true; // t && t returns true
        a2 = true && false; // t && f returns false
        a3 = false && true; // f && t returns false
        a4 = false && 3 === 4; // f && f returns false
        a5 = "Cat" && "Dog"; // t && t returns "Dog"
        a6 = false && "Cat"; // f && t returns false
        a7 = "Cat" && false; // t && f returns false
        a8 = "" && false; // f && f returns ""
        a9 = false && ""; // f && f returns false
    }} catch (e) {{
        console.error(`[testExpressions] fragment 134 error: ${e.message}`);
    }}

    // ---- fragment 135 ----
    try {{
        bCondition1 && bCondition2
    }} catch (e) {{
        console.error(`[testExpressions] fragment 135 error: ${e.message}`);
    }}

    // ---- fragment 136 ----
    try {{
        !(!bCondition1 || !bCondition2)
    }} catch (e) {{
        console.error(`[testExpressions] fragment 136 error: ${e.message}`);
    }}

    // ---- fragment 137 ----
    try {{
        bCondition1 || bCondition2
    }} catch (e) {{
        console.error(`[testExpressions] fragment 137 error: ${e.message}`);
    }}

    // ---- fragment 138 ----
    try {{
        !(!bCondition1 && !bCondition2)
    }} catch (e) {{
        console.error(`[testExpressions] fragment 138 error: ${e.message}`);
    }}

    // ---- fragment 139 ----
    try {{
        bCondition1 || (bCondition2 && bCondition3)
    }} catch (e) {{
        console.error(`[testExpressions] fragment 139 error: ${e.message}`);
    }}

    // ---- fragment 140 ----
    try {{
        bCondition1 || bCondition2 && bCondition3
    }} catch (e) {{
        console.error(`[testExpressions] fragment 140 error: ${e.message}`);
    }}

    // ---- fragment 141 ----
    try {{
        const a = 3;
        const b = -2;

        console.log(a > 0 || b > 0);
    }} catch (e) {{
        console.error(`[testExpressions] fragment 141 error: ${e.message}`);
    }}

    // ---- fragment 142 ----
    try {{
        x || y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 142 error: ${e.message}`);
    }}

    // ---- fragment 143 ----
    try {{
        function A() {
          console.log("called A");
          return false;
        }
        function B() {
          console.log("called B");
          return true;
        }

        console.log(B() || A());
        // Logs "called B" due to the function call,
        // then logs true (which is the resulting value of the operator)
    }} catch (e) {{
        console.error(`[testExpressions] fragment 143 error: ${e.message}`);
    }}

    // ---- fragment 144 ----
    try {{
        true || false && false; // returns true, because && is executed first
        (true || false) && false; // returns false, because grouping has the highest precedence
    }} catch (e) {{
        console.error(`[testExpressions] fragment 144 error: ${e.message}`);
    }}

    // ---- fragment 145 ----
    try {{
        true || true; // t || t returns true
        false || true; // f || t returns true
        true || false; // t || f returns true
        false || 3 === 4; // f || f returns false
        "Cat" || "Dog"; // t || t returns "Cat"
        false || "Cat"; // f || t returns "Cat"
        "Cat" || false; // t || f returns "Cat"
        "" || false; // f || f returns false
        false || ""; // f || f returns ""
        false || varObject; // f || object returns varObject
    }} catch (e) {{
        console.error(`[testExpressions] fragment 145 error: ${e.message}`);
    }}

    // ---- fragment 146 ----
    try {{
        bCondition1 && bCondition2
    }} catch (e) {{
        console.error(`[testExpressions] fragment 146 error: ${e.message}`);
    }}

    // ---- fragment 147 ----
    try {{
        !(!bCondition1 || !bCondition2)
    }} catch (e) {{
        console.error(`[testExpressions] fragment 147 error: ${e.message}`);
    }}

    // ---- fragment 148 ----
    try {{
        bCondition1 || bCondition2
    }} catch (e) {{
        console.error(`[testExpressions] fragment 148 error: ${e.message}`);
    }}

    // ---- fragment 149 ----
    try {{
        !(!bCondition1 && !bCondition2)
    }} catch (e) {{
        console.error(`[testExpressions] fragment 149 error: ${e.message}`);
    }}

    // ---- fragment 150 ----
    try {{
        bCondition1 && (bCondition2 || bCondition3)
    }} catch (e) {{
        console.error(`[testExpressions] fragment 150 error: ${e.message}`);
    }}

    // ---- fragment 151 ----
    try {{
        !(!bCondition1 || !bCondition2 && !bCondition3)
    }} catch (e) {{
        console.error(`[testExpressions] fragment 151 error: ${e.message}`);
    }}

    // ---- fragment 152 ----
    try {{
        let a = 3;

        console.log((a %= 2));

        console.log((a %= 0));

        console.log((a %= "hello"));
    }} catch (e) {{
        console.error(`[testExpressions] fragment 152 error: ${e.message}`);
    }}

    // ---- fragment 153 ----
    try {{
        x %= y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 153 error: ${e.message}`);
    }}

    // ---- fragment 154 ----
    try {{
        let bar = 5;

        bar %= 2; // 1
        bar %= "foo"; // NaN
        bar %= 0; // NaN

        let foo = 3n;
        foo %= 2n; // 1n
    }} catch (e) {{
        console.error(`[testExpressions] fragment 154 error: ${e.message}`);
    }}

    // ---- fragment 155 ----
    try {{
        let a = 2;

        console.log((a -= 3));

        console.log((a -= "Hello"));
    }} catch (e) {{
        console.error(`[testExpressions] fragment 155 error: ${e.message}`);
    }}

    // ---- fragment 156 ----
    try {{
        x -= y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 156 error: ${e.message}`);
    }}

    // ---- fragment 157 ----
    try {{
        let bar = 5;

        bar -= 2; // 3
    }} catch (e) {{
        console.error(`[testExpressions] fragment 157 error: ${e.message}`);
    }}

    // ---- fragment 158 ----
    try {{
        bar -= "foo"; // NaN
    }} catch (e) {{
        console.error(`[testExpressions] fragment 158 error: ${e.message}`);
    }}

    // ---- fragment 159 ----
    try {{
        let foo = 3n;
        foo -= 2n; // 1n
        foo -= 1; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[testExpressions] fragment 159 error: ${e.message}`);
    }}

    // ---- fragment 160 ----
    try {{
        let a = 3;

        console.log((a **= 2));

        console.log((a **= 0));

        console.log((a **= 'hello'));
    }} catch (e) {{
        console.error(`[testExpressions] fragment 160 error: ${e.message}`);
    }}

    // ---- fragment 161 ----
    try {{
        x **= y
    }} catch (e) {{
        console.error(`[testExpressions] fragment 161 error: ${e.message}`);
    }}

    // ---- fragment 162 ----
    try {{
        let bar = 5;
        bar **= 2; // 25
    }} catch (e) {{
        console.error(`[testExpressions] fragment 162 error: ${e.message}`);
    }}

    // ---- fragment 163 ----
    try {{
        let baz = 5;
        baz **= "foo"; // NaN
    }} catch (e) {{
        console.error(`[testExpressions] fragment 163 error: ${e.message}`);
    }}

    // ---- fragment 164 ----
    try {{
        let foo = 3n;
        foo **= 2n; // 9n
        foo **= 1; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }} catch (e) {{
        console.error(`[testExpressions] fragment 164 error: ${e.message}`);
    }}

    // ---- fragment 165 ----
    try {{
        function getVowels(str) {
          const m = str.match(/[aeiou]/gi);
          if (m === null) {
            return 0;
          }
          return m.length;
        }

        console.log(getVowels("sky"));
    }} catch (e) {{
        console.error(`[testExpressions] fragment 165 error: ${e.message}`);
    }}

    // ---- fragment 166 ----
    try {{
        null
    }} catch (e) {{
        console.error(`[testExpressions] fragment 166 error: ${e.message}`);
    }}

    // ---- fragment 167 ----
    try {{
        typeof null; // "object" (not "null" for legacy reasons)
        typeof undefined; // "undefined"
        null === undefined; // false
        null == undefined; // true
        null === null; // true
        null == null; // true
        !null; // true
        Number.isNaN(1 + null); // false
        Number.isNaN(1 + undefined); // true
    }} catch (e) {{
        console.error(`[testExpressions] fragment 167 error: ${e.message}`);
    }}

}
module.exports = { testExpressions };