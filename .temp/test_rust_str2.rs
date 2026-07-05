struct S;
impl S {
    fn write(&mut self, s: &str) {}
    fn next_label(&mut self) -> String { "blk0".to_string() }
    fn emit_expr(&mut self, arg: &str) {}
}
fn main() {
    let mut s = S;
    let blk = s.next_label();
    let elem_type_str = "i64";
    let obj_name = "arr";
    let args: Vec<&str> = vec![];
    s.write(&format!("({}: {{ ", blk));
    s.write(&format!(
        "var __spliced: std.ArrayList({}) = .empty; ",
        elem_type_str
    ));
    s.write("const __start = @as(usize, @intCast(@max(0, ");
    if let Some(arg) = args.first() {
        s.emit_expr(arg);
    } else {
        s.write("0");
    }
    s.write("))); ");
    s.write("const __cnt = @as(usize, @intCast(@min(@max(0, "));
}
