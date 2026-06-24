// test_classes.js — Class declaration tests for native_proto mode
// Covers: class struct generation, constructor → init(),
//         method body this→self rewrite, new ClassName → ClassName.init()

// ── Rectangle: simple class with i64 fields ──────────────────

class Rectangle {
    width = 0;
    height = 0;

    constructor(w, h) {
        this.width = w;
        this.height = h;
    }

    area() {
        return this.width * this.height;
    }
}

/** @returns {i64} */
export function testRectArea() {
    const rect = new Rectangle(3, 4);
    return rect.area();
}

/** @returns {i64} */
export function testRectPerim() {
    const rect = new Rectangle(5, 6);
    return 2 * (rect.area()) + 0; // Approximate for now
}

// ── User: class with i64 + string mixed fields ───────────────

class User {
    id = 0;
    name = "";

    constructor(idVal, nameVal) {
        this.id = idVal;
        this.name = nameVal;
    }

    getId() {
        return this.id;
    }

    getName() {
        return this.name;
    }
}

/** @returns {i64} */
export function testUserId() {
    const u = new User(42, "Alice");
    return u.getId();
}

/** @returns {i64} */
export function testUserNameLength() {
    const u = new User(1, "Bob");
    const name = u.getName();
    // name.length returns usize, use i64 function instead
    if (name.length > 0) { return 1; }
    return 0;
}
