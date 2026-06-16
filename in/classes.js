// Class declaration test
class Rectangle {
    constructor(w, h) {
        this.width = w;
        this.height = h;
    }

    area() {
        return this.width * this.height;
    }
}

function useRectangle() {
    const rect = new Rectangle(10, 5);
    return rect.area();
}

// test_ variable for Zig test generation
const test_useRectangle_classes = useRectangle();

export { Rectangle, useRectangle };
