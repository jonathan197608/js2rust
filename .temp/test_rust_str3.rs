struct S;
impl S {
    fn write(&mut self, s: &str) {}
    fn next_label(&mut self) -> String { "blk0".to_string() }
}
fn main() {
    let mut s = S;
    s.write(&format!("const __cnt = @as(usize, @intCast(@min(@max(0, "));
    println!("ok");
}
