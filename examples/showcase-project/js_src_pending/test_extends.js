// BUG-14: `class extends` / `super` — extends is tracked for instanceof only,
// super produces @compileError("super not supported"). No field/method
// inheritance is generated. Zig structs are always flat.
// Status: BLOCKED by architecture limitation. Enable when BUG-14 is fixed.

/** @returns {i64} */
export function testClassExtends() {
    class Animal {
        constructor(name) {
            this.name = name;
        }
        speak() {
            return this.name;
        }
    }
    class Dog extends Animal {
        constructor(name) {
            super(name);
        }
        bark() {
            return this.name;
        }
    }
    const d = new Dog("Rex");
    if (d.bark() === "Rex") {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testClassExtendsMethodOverride() {
    class Shape {
        describe() {
            return "shape";
        }
    }
    class Circle extends Shape {
        describe() {
            return "circle";
        }
    }
    const c = new Circle();
    if (c.describe() === "circle") {
        return 1;
    }
    return 0;
}
