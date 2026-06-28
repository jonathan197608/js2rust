// test_expressions_part1.js — Basic expression tests (MDN reference).
// Only features that js2rust definitely supports.

export function testExpressionsPart1() {
    // ---- arithmetic ----
    console.log("1+2 = " + (1 + 2));
    console.log("3*4 = " + (3 * 4));
    console.log("10-3 = " + (10 - 3));
    console.log("20/5 = " + (20 / 5));
    console.log("10%3 = " + (10 % 3));
    console.log("2**3 = " + (2 ** 3));

    // ---- comparison ----
    console.log("3 > 2: " + (3 > 2));
    console.log("3 < 2: " + (3 < 2));
    console.log("3 >= 3: " + (3 >= 3));
    console.log("3 <= 2: " + (3 <= 2));

    // ---- equality (same-type only) ----
    console.log("1 == 1: " + (1 == 1));
    console.log("1 === 1: " + (1 === 1));
    console.log("1 !== 2: " + (1 !== 2));
    console.log("'a' == 'a': " + ("a" == "a"));
    console.log("true === true: " + (true === true));

    // ---- logical ----
    console.log("true && false: " + (true && false));
    console.log("true || false: " + (true || false));
    console.log("!true: " + (!true));

    // ---- ternary ----
    console.log("1 > 2 ? 'a' : 'b' = " + (1 > 2 ? "a" : "b"));

    // ---- Array (property access only) ----
    let arr = [10, 20, 30];
    console.log("arr.length = " + arr.length);

    // ---- String (property access only) ----
    let s = "Hello World";
    console.log("s.length = " + s.length);

    return 0;
}
