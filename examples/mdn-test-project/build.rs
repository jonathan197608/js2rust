// build.rs — Transpile JS to Zig during build
fn main() {
    js2rust_bridge::build(false);
}
