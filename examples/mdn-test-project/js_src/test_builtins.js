// MDN JS Reference Tests — Builtins
// Tests supported builtin objects: Math, Array, String (with charAt/toUpperCase/toLowerCase),
// Map, Set, Object, parseInt
//
// NOTE: Now that String methods emit `try` properly, charAt/toUpperCase/toLowerCase
//       work correctly in both standalone calls and template literals.

export function testBuiltins() {
    // ── 1. Math Methods ──
    {
        try {
            console.log(`Math.abs(-5) = ${Math.abs(-5)}`);
            console.log(`Math.floor(3.7) = ${Math.floor(3.7)}`);
            console.log(`Math.ceil(3.2) = ${Math.ceil(3.2)}`);
            console.log(`Math.round(3.5) = ${Math.round(3.5)}`);
            console.log(`Math.max(10, 20) = ${Math.max(10, 20)}`);
            console.log(`Math.min(10, 20) = ${Math.min(10, 20)}`);
            console.log(`Math.pow(2, 10) = ${Math.pow(2, 10)}`);
        } catch (e) {
            console.log("[Math] error");
        }
    }

    // ── 2. Array Methods ──
    {
        try {
            const arr = [1, 2, 3, 4, 5];
            console.log(`arr.length = ${arr.length}`);
            console.log(`arr.indexOf(3) = ${arr.indexOf(3)}`);
            if (arr.includes(5)) { console.log("arr.includes(5) = true"); } else { console.log("arr.includes(5) = false"); }
        } catch (e) {
            console.log("[Array] error");
        }
    }

    // ── 3. String Methods ──
    {
        try {
            const str = "Hello, World";
            console.log(`str.length = ${str.length}`);
            console.log(`str.indexOf("World") = ${str.indexOf("World")}`);
            if (str.startsWith("Hello")) { console.log("startsWith Hello: true"); } else { console.log("startsWith Hello: false"); }
            if (str.endsWith("World")) { console.log("endsWith World: true"); } else { console.log("endsWith World: false"); }
        } catch (e) {
            console.log("[String] error");
        }
    }

    // ── 4. String Case Methods ──
    {
        try {
            const s = "hello";
            const up = s.toUpperCase();
            console.log(`toUpperCase: ${up}`);
            const lo = "WORLD".toLowerCase();
            console.log(`toLowerCase: ${lo}`);
        } catch (e) {
            console.log("[String case] error");
        }
    }

    // ── 5. String charAt ──
    {
        try {
            const s = "abcdef";
            const c0 = s.charAt(0);
            const c3 = s.charAt(3);
            console.log(`charAt(0) = ${c0}`);
            console.log(`charAt(3) = ${c3}`);
        } catch (e) {
            console.log("[charAt] error");
        }
    }

    // ── 6. Map Methods ──
    {
        try {
            const m = new Map();
            m.set("a", 1);
            m.set("b", 2);
            const v = m.get("a");
            if (v === 1) { console.log("Map.get(a) = 1"); }
            if (m.has("b")) { console.log("Map.has(b) = true"); }
            console.log(`Map.size = ${m.size}`);
        } catch (e) {
            console.log("[Map] error");
        }
    }

    // ── 7. Set Methods ──
    {
        try {
            const s = new Set();
            s.add(10);
            s.add(20);
            s.add(30);
            if (s.has(20)) { console.log("Set.has(20) = true"); }
            console.log(`Set.size = ${s.size}`);
        } catch (e) {
            console.log("[Set] error");
        }
    }

    // ── 8. Object.keys ──
    {
        try {
            const obj = { a: 1, b: 2 };
            if (Object.keys(obj).length === 2) {
                console.log("Object.keys length = 2");
            } else {
                console.log("Object.keys length != 2");
            }
        } catch (e) {
            console.log("[Object] error");
        }
    }

    // ── 9. parseInt ──
    {
        try {
            const n = parseInt("42");
            console.log(`parseInt("42") = ${n}`);
        } catch (e) {
            console.log("[parseInt] error");
        }
    }

    return 0;
}
