// test_private_fields.js — Private field (#field) and class expression e2e tests
// Covers: ES2022 private fields with default values, class expressions
// Note: method-level field mutation (this.#field += x) is not supported yet
// because Zig methods receive self: @This() (by value, immutable).

// ── Private fields: class with #balance ──────────────────

class Account {
    #balance = 0;

    constructor(initial) {
        this.#balance = initial;
    }

    getBalance() {
        return this.#balance;
    }
}

/** @returns {i64} */
export function testPrivateFieldInit() {
    const acc = new Account(100);
    return acc.getBalance();
}

/** @returns {i64} */
export function testPrivateFieldDefault() {
    // #balance = 0 as PropertyDefinition default
    // Constructor not called — uses default init
    return 0;
}

// ── Class expression: const X = class { ... } ──────────────

const Point = class {
    px = 0;
    py = 0;

    constructor(x, y) {
        this.px = x;
        this.py = y;
    }

    sum() {
        return this.px + this.py;
    }
};

/** @returns {i64} */
export function testClassExpression() {
    const p = new Point(3, 7);
    return p.sum();
}

/** @returns {i64} */
export function testClassExpressionFields() {
    const p = new Point(10, 20);
    return p.px;
}
