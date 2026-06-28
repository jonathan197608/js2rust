// MDN JS Reference Tests — Expressions
// Tests supported expression types: arithmetic, comparison, logical,
// bitwise, unary, ternary, template literals, array/object literals,
// string concatenation, compound assignment, parenthesized
//
// NOTE: Avoid complex expressions inside ${} in template literals.
//       The transpiler uses {} (default format) for expressions it
//       can't type-infer, which fails on Zig 0.16. Only interpolate
//       simple const variables with literal initializers, or use
//       literal expressions directly.

export function testExpressions() {
    // ── 1. Arithmetic Operations ──
    {
        try {
            console.log(`5 + 3 = ${5 + 3}`);
            console.log(`10 - 4 = ${10 - 4}`);
            console.log(`6 * 7 = ${6 * 7}`);
            console.log(`20 / 4 = ${20 / 4}`);
            console.log(`17 % 5 = ${17 % 5}`);
            console.log(`2 ** 10 = ${2 ** 10}`);
        } catch (e) {
            console.log("[arithmetic] error");
        }
    }

    // ── 2. Comparison Operations ──
    {
        try {
            if (5 === 5) { console.log("5 === 5: true"); } else { console.log("5 === 5: false"); }
            if (5 !== 3) { console.log("5 !== 3: true"); } else { console.log("5 !== 3: false"); }
            if (10 > 5) { console.log("10 > 5: true"); } else { console.log("10 > 5: false"); }
            if (5 >= 5) { console.log("5 >= 5: true"); } else { console.log("5 >= 5: false"); }
            if (3 < 7) { console.log("3 < 7: true"); } else { console.log("3 < 7: false"); }
            if (3 <= 3) { console.log("3 <= 3: true"); } else { console.log("3 <= 3: false"); }
        } catch (e) {
            console.log("[comparison] error");
        }
    }

    // ── 3. Logical Operations ──
    {
        try {
            if (true && false) { console.log("true && false: true"); } else { console.log("true && false: false"); }
            if (true || false) { console.log("true || false: true"); } else { console.log("true || false: false"); }
            if (!false) { console.log("!false: true"); } else { console.log("!false: false"); }
            if (5 > 3 && 10 > 5) { console.log("5>3 && 10>5: true"); } else { console.log("5>3 && 10>5: false"); }
            if (5 > 10 || 3 > 1) { console.log("5>10 || 3>1: true"); } else { console.log("5>10 || 3>1: false"); }
        } catch (e) {
            console.log("[logical] error");
        }
    }

    // ── 4. Bitwise Operations ──
    {
        try {
            console.log(`12 & 10 = ${12 & 10}`);
            console.log(`12 | 10 = ${12 | 10}`);
            console.log(`12 ^ 10 = ${12 ^ 10}`);
            console.log(`1 << 4 = ${1 << 4}`);
            console.log(`256 >> 2 = ${256 >> 2}`);
        } catch (e) {
            console.log("[bitwise] error");
        }
    }

    // ── 5. Unary Operations ──
    {
        try {
            console.log(`-42 = ${-42}`);
            console.log(`-5 = ${-5}`);
            if (!true) { console.log("!true: true"); } else { console.log("!true: false"); }
        } catch (e) {
            console.log("[unary] error");
        }
    }

    // ── 6. Ternary Operator ──
    {
        try {
            if (5 > 3) { console.log("5 > 3 ? yes"); } else { console.log("5 > 3 ? no"); }
            if (10 > 20) { console.log("10 > 20 ? big"); } else { console.log("10 > 20 ? small"); }
        } catch (e) {
            console.log("[ternary] error");
        }
    }

    // ── 7. Template Literals ──
    {
        try {
            const name = "World";
            console.log(`Hello, ${name}!`);
            const a = 10;
            const b = 20;
            console.log(`${a} + ${b} = 30`);
        } catch (e) {
            console.log("[template] error");
        }
    }

    // ── 8. Array Literals ──
    {
        try {
            const arr = [10, 20, 30];
            console.log(`arr.length = ${arr.length}`);
        } catch (e) {
            console.log("[array] error");
        }
    }

    // ── 9. Object Literals & Property Access ──
    {
        try {
            const obj = { x: 10, y: 20, z: 30 };
            console.log(`obj.x = ${obj.x}`);
            console.log(`obj.y = ${obj.y}`);
            console.log(`obj.z = ${obj.z}`);
        } catch (e) {
            console.log("[object] error");
        }
    }

    // ── 10. String Concatenation ──
    {
        try {
            const greeting = "Hello" + ", " + "World" + "!";
            console.log(greeting);
            const s = "abc" + "def";
            console.log(s);
        } catch (e) {
            console.log("[string concat] error");
        }
    }

    // ── 11. Compound Assignment ──
    {
        try {
            let x = 10;
            x += 5;
            console.log(`x = ${x}`);
            x *= 2;
            console.log(`x = ${x}`);
            x -= 10;
            console.log(`x = ${x}`);
        } catch (e) {
            console.log("[compound] error");
        }
    }

    // ── 12. Parenthesized Expressions ──
    {
        try {
            console.log(`(2 + 3) * 4 = ${(2 + 3) * 4}`);
            console.log(`2 + 3 * 4 = ${2 + 3 * 4}`);
            console.log(`((1 + 2) * (3 + 4)) = ${((1 + 2) * (3 + 4))}`);
        } catch (e) {
            console.log("[parenthesized] error");
        }
    }

    return 0;
}
