fn main() {
    js2rust_bridge::build(js2rust_bridge::BuildConfig {
        name: "main".into(),
        js_file: "js_src/main.js".into(),
        additional_js_files: vec![],
        host_functions: None,
        force_rebuild: false,
    });
}
