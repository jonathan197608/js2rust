// T-FN-05: class constructor + instance method
class Rectangle {
    constructor(w, h) {
        this.width = w;
        this.height = h;
    }
    area() {
        return this.width * this.height;
    }
}
function testRect() {
    const r = new Rectangle(3, 4);
    return r.area();
}
const test_rect = testRect(); // => 12

// T-FN-10: constructor with field defaults
class Settings {
    constructor() {
        this.volume = 50;
        this.brightness = 75;
    }
    getVolume() { return this.volume; }
    getBrightness() { return this.brightness; }
}
function testSettings() {
    const s = new Settings();
    return s.getVolume();
}
const test_settings = testSettings(); // => 50

// T-FN-05b: multiple instance methods
class Counter {
    constructor(start) {
        this.count = start;
    }
    get() { return this.count; }
}
function testCounter() {
    const c = new Counter(100);
    return c.get();
}
const test_counter = testCounter(); // => 100

// T-EXPR-26b: class method chain
class Vec2 {
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }
    sum() { return this.x + this.y; }
}
function testVec2() {
    const v = new Vec2(3, 7);
    return v.sum();
}
const test_vec2 = testVec2(); // => 10
