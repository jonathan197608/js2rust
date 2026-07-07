// test_static_fields.js — Static field read/write tests
// Covers: ClassName.field read, ClassName.field = value write, computed values
// Note: static {} block runtime execution is validated in unit tests (ast-check);
// runtime execution requires an init-function mechanism not yet in the bridge layer.

// ── Counter: static field read + write ────────────────────────

class Counter {
    static count = 0;
}

/** @returns {i64} */
export function testStaticFieldRead() {
    if (Counter.count === 0) { return 0; }
    return -1;
}

/** @returns {i64} */
export function testStaticFieldAssign() {
    Counter.count = 10;
    if (Counter.count === 10) { return 0; }
    return -1;
}

// ── Multiplier: static field with initial value ───────────────

class Multiplier {
    static factor = 3;
}

/** @returns {i64} */
export function testStaticFieldMultiply() {
    if (Multiplier.factor * 7 === 21) { return 0; }
    return -1;
}

// ── State: static field modified then read by another function ──

class State {
    static value = 5;
}

/** @returns {i64} */
export function testStaticFieldSetThenGet() {
    State.value = 42;
    return State.value;
}
