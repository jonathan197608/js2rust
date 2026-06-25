// Math 高级方法转译测试
// 编译: js2zig math_advanced.js -o math_advanced.zig

export function testExpm1() {
    return Math.expm1(1.0); // should return e^1 - 1 ≈ 1.718
}

export function testSinh() {
    return Math.sinh(1.0); // should return ≈ 1.175
}

export function testAsinh() {
    return Math.asinh(1.0); // should return ≈ 0.881
}

export function testClz32() {
    return Math.clz32(0x00FF0000); // should return 8
}

export function testFround() {
    return Math.fround(1.5); // should return 1.5 as f32
}

export function testImul() {
    return Math.imul(2, 3); // should return 6
}

export function testLog1p() {
    return Math.log1p(1.0); // should return ln(2) ≈ 0.693
}
