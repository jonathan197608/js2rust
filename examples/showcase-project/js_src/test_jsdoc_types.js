// test_jsdoc_types.js
// End-to-end tests for complex JSDoc type annotations (Section 2.18).
// Tests: @type {number[]}, @type {{name:string}}, @returns {{name:string}},
//        @typedef, @property
//
// Strategy: Use these JSDoc patterns internally in export functions that
// return i64, so the C ABI binding is simple and testable.

// ══════════════════════════════════════════════════════════════
// @type {number[]} — array type annotation
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testJsdocArrayLength() {
    /**
     * @type {number[]}
     */
    const arr = [10, 20, 30, 40, 50];
    if (arr.length === 5) {
        return 1;
    }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// @type {{name: string, age: number}} — anonymous object type
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testJsdocAnonObject() {
    /**
     * @type {{name: string, age: number}}
     */
    const user = { name: "Alice", age: 30 };
    if (user.age === 30) {
        return 1;
    }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// @typedef — named type alias
// @property — typedef property definitions
// ══════════════════════════════════════════════════════════════

/**
 * @typedef {Object} Point
 * @property {number} x
 * @property {number} y
 */

/**
 * @returns {i64}
 */
export function testJsdocTypedef() {
    /**
     * @type {Point}
     */
    const p = { x: 3, y: 4 };
    const sum = p.x + p.y;
    if (sum === 7) {
        return 1;
    }
    return 0;
}

// NOTE: @returns {{name: string, age: number}} on non-export function
// generates `.{ .name = []const u8, .age = i64 }` as return type —
// this is struct literal syntax, not a valid Zig type. Codegen bug.
// Covered by Rust unit test (IR text only) + ast-check, but e2e BLOCKED.
