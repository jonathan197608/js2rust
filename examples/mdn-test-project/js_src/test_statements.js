// MDN JS Reference Tests — Statements
// Tests supported statement types: variable declarations, if-else,
// switch, for (with non-zero init), for-of, while, do-while, break,
// continue, try-catch-throw, block scoping, nested loops
//
// NOTE: for loop now supports non-zero init (e.g. let i = 1)
//       thanks to the fix in emit_for_body() which now emits the
//       actual initializer expression instead of hardcoding 0.

export function testStatements() {
    // ── 1. Variable Declarations ──
    {
        try {
            let x = 10;
            const y = 20;
            console.log(`x = ${x}, y = ${y}`);
            let z = x + y;
            console.log(`z = ${z}`);
        } catch (e) {
            console.log("[var decl] error");
        }
    }

    // ── 2. If-Else Chains ──
    {
        try {
            const score = 85;
            if (score >= 90) {
                console.log("Grade: A");
            } else if (score >= 80) {
                console.log("Grade: B");
            } else if (score >= 70) {
                console.log("Grade: C");
            } else {
                console.log("Grade: F");
            }
        } catch (e) {
            console.log("[if-else] error");
        }
    }

    // ── 3. Switch (integer cases) ──
    {
        try {
            const day = 3;
            switch (day) {
                case 1: console.log("Monday"); break;
                case 2: console.log("Tuesday"); break;
                case 3: console.log("Wednesday"); break;
                case 4: console.log("Thursday"); break;
                case 5: console.log("Friday"); break;
                default: console.log("Weekend"); break;
            }
        } catch (e) {
            console.log("[switch] error");
        }
    }

    // ── 4. For Loop (with non-zero init, now fixed) ──
    {
        try {
            let sum = 0;
            for (let i = 1; i <= 5; i++) {
                sum = sum + i;
            }
            console.log(`Sum 1..5 = ${sum}`);
        } catch (e) {
            console.log("[for] error");
        }
    }

    // ── 5. For-Of Loop ──
    {
        try {
            const arr = [10, 20, 30];
            let total = 0;
            for (const item of arr) {
                total = total + item;
            }
            console.log(`For-of total = ${total}`);
        } catch (e) {
            console.log("[for-of] error");
        }
    }

    // ── 6. While Loop ──
    {
        try {
            let n = 16;
            let count = 0;
            while (n > 1) {
                n = n / 2;
                count = count + 1;
            }
            console.log(`log2(16) = ${count}`);
        } catch (e) {
            console.log("[while] error");
        }
    }

    // ── 7. Do-While Loop ──
    {
        try {
            let i = 0;
            do {
                console.log(`do-while iteration ${i}`);
                i = i + 1;
            } while (i < 3);
        } catch (e) {
            console.log("[do-while] error");
        }
    }

    // ── 8. Break (now with i starting from 1) ──
    {
        try {
            for (let i = 1; i <= 10; i++) {
                if (i > 5) { break; }
                console.log(`break test: ${i}`);
            }
        } catch (e) {
            console.log("[break] error");
        }
    }

    // ── 9. Continue (now with i starting from 1) ──
    {
        try {
            for (let i = 1; i <= 5; i++) {
                if (i % 2 === 0) { continue; }
                console.log(`odd: ${i}`);
            }
        } catch (e) {
            console.log("[continue] error");
        }
    }

    // ── 10. Try-Catch-Throw ──
    {
        try {
            try {
                throw "error";
            } catch (e) {
                console.log("caught error");
            }
        } catch (e) {
            console.log("[try-catch] error");
        }
    }

    // ── 11. Block Scoping ──
    {
        try {
            {
                const x = 100;
                console.log(`block1 x = ${x}`);
            }
            {
                const x = 200;
                console.log(`block2 x = ${x}`);
            }
        } catch (e) {
            console.log("[block] error");
        }
    }

    // ── 12. Nested Loops (starting from 1) ──
    {
        try {
            for (let i = 1; i <= 3; i++) {
                for (let j = 1; j <= 3; j++) {
                    console.log(`i=${i}, j=${j}`);
                }
            }
        } catch (e) {
            console.log("[nested] error");
        }
    }

    return 0;
}
