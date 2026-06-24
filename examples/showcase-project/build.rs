// build.rs — Transpile JS to Zig during build
use js2rust_bridge::transpile_js_dir;

fn main() {
    // Transpile JS files in js_src/ to Zig
    transpile_js_dir("js_src", "src/gen");
}
