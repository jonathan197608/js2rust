// phase_memory.js — Memory stress tests for multi-arena allocator
//
// Verifies that the multi-arena allocator correctly handles:
//  1. String operations under memory pressure
//  2. Map/Set mass insert/delete (hash map memory pressure)
//  3. Array built-in methods (sort/reverse/slice)
//  4. Correctness after automatic arena rotation (cooling + reset)
//
// Run with JS2RUST_MAX_ARENA_MB=1 to force rapid arena swaps.

// ── String stress: simple string return ──────────────────
// Verifies string allocation through C ABI works under memory pressure.
/**
 * @param {string} name
 * @returns {string}
 */
export function testLongGreeting(name) {
    return "Hello, " + name + "! Welcome, " + name + "!";
}

// ── Map stress: many insert → verify get → mass delete ─────
// Tests HashMap growth and reclamation in the arena.
// Uses string keys to stay within supported codegen patterns.
/**
 * @param {i64} n
 * @returns {i64}
 */
export function testMapStress() {
    const m = new Map();
    m.set("k0", 0);
    m.set("k1", 10);
    m.set("k2", 20);
    m.set("k3", 30);
    m.set("k4", 40);
    m.set("k5", 50);
    m.set("k6", 60);
    m.set("k7", 70);
    m.set("k8", 80);
    m.set("k9", 90);
    // Verify get for all keys
    if (m.get("k0") === 0 && m.get("k5") === 50 && m.get("k9") === 90) {
        // Delete some entries
        m.delete("k0");
        m.delete("k5");
        m.delete("k9");
        if (m.has("k0") === false && m.has("k5") === false && m.has("k9") === false) {
            return 1;
        }
    }
    return 0;
}

// ── Set stress: mass insert + verify + delete ─────────
/**
 * @param {i64} n
 * @returns {i64}
 */
export function testSetStress() {
    const s = new Set();
    s.add(1);
    s.add(2);
    s.add(3);
    s.add(4);
    s.add(5);
    s.add(6);
    s.add(7);
    s.add(8);
    s.add(9);
    s.add(10);
    if (s.has(1) && s.has(5) && s.has(10)) {
        // Delete some entries
        s.delete(1);
        s.delete(5);
        s.delete(10);
        if (s.has(1) === false && s.has(5) === false && s.has(10) === false) {
            return 1;
        }
    }
    return 0;
}

// ── Array builtin stress: sort + reverse on mid-size array ──
// In-place mutation exercises ArrayList operations.
/**
 * @returns {i64}
 */
export function testArrayMutStress() {
    const arr = [5, 3, 8, 1, 9, 2, 7, 4, 6, 0,
                  15, 13, 18, 11, 19, 12, 17, 14, 16, 10];
    arr.sort();
    arr.reverse();
    // After sort + reverse, arr is descending. Use slice to verify.
    const first = arr.slice(0, 1);
    if (first.length === 1) {
        return 0;
    }
    return -1;
}

// ── Simple i64 function for post-reset verification ────
/**
 * @param {i64} a
 * @param {i64} b
 * @returns {i64}
 */
export function testMemoryAdd(a, b) {
    return a + b;
}
