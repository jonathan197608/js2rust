struct S;
impl S { fn write(&mut self, s: &str) {} }
fn main() {
    let mut s = S;
    s.write("const __cnt = @as(usize, @intCast(@min(@max(0, "));
}
