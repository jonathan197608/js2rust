// test_static_fields.js — Static field read/write + static {} block tests
// Covers: ClassName.field read, ClassName.field = value write, computed values,
//         static {} block execution at runtime (via init_js2rust mechanism)

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

// ── Static block: runtime execution test ──────────────────────

class Config {
    static timeout = 0;
    static {
        Config.timeout = 5000;
    }
}

/** @returns {i64} */
export function testStaticBlockInit() {
    if (Config.timeout === 5000) { return 0; }
    return -1;
}

// ── Static block: `this` reference ────────────────────────────

class Registry {
    static entries = 0;
    static {
        this.entries = 100;
    }
}

/** @returns {i64} */
export function testStaticBlockThis() {
    if (Registry.entries === 100) { return 0; }
    return -1;
}
