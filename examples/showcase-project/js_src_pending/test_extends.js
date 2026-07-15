// BUG-14: `class extends` is now a compile error.
// `class Child extends Parent` generates:
//   @compileError("class extends is not supported: use composition instead")
// This is by design — Zig structs are always flat and cannot model
// prototype-based inheritance. Use composition (embedding a parent
// struct as a field) instead.
// Status: WONTFIX (compile error by design).

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
