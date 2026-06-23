fn main() {
    js2rust_bridge::build(js2rust_bridge::BuildConfig {
        name: "app".into(),
        js_file: "js_src/app.js".into(),
        additional_js_files: vec![
            "js_src/phase5.js".into(),
            "js_src/test_throw.js".into(),
        ],
        host_functions: None,
        force_rebuild: false,
    });
}
