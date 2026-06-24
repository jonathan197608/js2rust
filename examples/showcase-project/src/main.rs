// main.rs — Showcase for for-in static struct feature
// This example demonstrates that for-in loops with static structs
// are unrolled at compile time into one block per field.

mod gen;

use gen::*;

pub fn main() !void {
    // Demo: for-in with static struct
    const result = demoForInStruct();
    std.debug.print("demoForInStruct() = {s}\n", .{result});
}
