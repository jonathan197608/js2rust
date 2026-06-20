const std = @import("std");

pub fn printTypeInfo(comptime T: type) void {
    const info = @typeInfo(T);
    std.debug.print("Type: {s}, typeInfo: {}\n", .{ @typeName(T), info });
}

pub fn main() void {
    printTypeInfo(comptime_int);
    printTypeInfo(comptime_float);
    printTypeInfo(*const [5:0]u8);
}
