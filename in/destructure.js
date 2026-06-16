// destructure.js — Test destructuring syntax support

// === Object destructuring ===
function usePoint() {
    const p = { x: 10, y: 20 };
    const { x, y } = p;
    return x + y;
}

// === Renamed object destructuring ===
function useRect() {
    const r = { width: 100, height: 50 };
    const { width: w, height: h } = r;
    return w * h;
}

// === Array destructuring ===
function useRGB() {
    const arr = [255, 128, 0];
    const [r, g, b] = arr;
    return r + g + b;
}

// test_ variables — generate Zig test assertions
const test_usePoint = usePoint();
const test_useRect = useRect();
const test_useRGB = useRGB();
