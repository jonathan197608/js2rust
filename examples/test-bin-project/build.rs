fn main() {
    js2rust_bridge::build(js2rust_bridge::BuildConfig {
        name: "main".into(),
        js_file: "js_src/main.js".into(),
        additional_js_files: vec![],
        host_functions: Some(js2rust_bridge::HostConfig {
            functions: vec![
                js2rust_bridge::HostFunction {
                    name: "host_add".into(),
                    params: vec![js2rust_bridge::HostType::I64, js2rust_bridge::HostType::I64],
                    return_type: Some(js2rust_bridge::HostType::I64),
                    is_async: false,
                    async_return_fields: vec![],
                },
                js2rust_bridge::HostFunction {
                    name: "host_concat".into(),
                    params: vec![
                        js2rust_bridge::HostType::Str,
                        js2rust_bridge::HostType::Str,
                    ],
                    return_type: Some(js2rust_bridge::HostType::Str),
                    is_async: false,
                    async_return_fields: vec![],
                },
                js2rust_bridge::HostFunction {
                    name: "host_strlen".into(),
                    params: vec![js2rust_bridge::HostType::Str],
                    return_type: Some(js2rust_bridge::HostType::I64),
                    is_async: false,
                    async_return_fields: vec![],
                },
                js2rust_bridge::HostFunction {
                    name: "host_multiply".into(),
                    params: vec![js2rust_bridge::HostType::I64, js2rust_bridge::HostType::I64],
                    return_type: Some(js2rust_bridge::HostType::I64),
                    is_async: false,
                    async_return_fields: vec![],
                },
                js2rust_bridge::HostFunction {
                    name: "fetch_user".into(),
                    params: vec![js2rust_bridge::HostType::Str],
                    return_type: None, // struct return
                    is_async: true,
                    async_return_fields: vec![
                        ("id".into(), js2rust_bridge::HostType::I64),
                        ("name".into(), js2rust_bridge::HostType::Str),
                    ],
                },
            ],
        }),
        force_rebuild: false,
    });
}
