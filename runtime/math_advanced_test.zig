// Math 高级方法测试
// 编译: zig test math_advanced_test.zig

const std = @import("std");

// 这些函数应该在 js_number.zig 或内联生成
// 这里我们直接测试 Zig 标准库函数是否存在

test "Math.expm1" {
    const x: f64 = 1.0;
    const result = std.math.expm1(x);
    try std.testing.expect(result > 1.7 and result < 1.8); // e^1 - 1 ≈ 1.718
}

test "Math.sinh" {
    const x: f64 = 1.0;
    const result = std.math.sinh(x);
    try std.testing.expect(result > 1.17 and result < 1.18); // sinh(1) ≈ 1.175
}

test "Math.asinh" {
    const x: f64 = 1.0;
    const result = std.math.asinh(x);
    try std.testing.expect(result > 0.88 and result < 0.89); // asinh(1) ≈ 0.881
}

test "Math.clz32" {
    const x: u32 = 0x00FF0000;
    const result = @clz(x);
    try std.testing.expect(result == 8);
}

test "Math.fround" {
    const x: f64 = 1.0;
    const result: f32 = @as(f32, @floatCast(x));
    try std.testing.expect(result == 1.0);
}

test "Math.imul" {
    const a: i32 = 2;
    const b: i32 = 3;
    const result: i32 = a *% b; // 溢出乘法
    try std.testing.expect(result == 6);
}

test "Math.log1p" {
    const x: f64 = 1.0;
    const result = std.math.log1p(x);
    try std.testing.expect(result > 0.69 and result < 0.70); // ln(2) ≈ 0.693
}
