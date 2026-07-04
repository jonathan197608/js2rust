// Temporary: print full codegen and emitter outputs for arrow_simple
use js2zig_core::parser;
use js2zig_core::infer::TypeInferrer;
use js2zig_core::jsdoc;
use js2zig_core::types::JSDocData;
use js2zig_core::zigir::lower::Lowerer;
use js2zig_core::zigir::emit::Emitter;
use std::collections::HashSet;

fn main() {
    let js = r#"
/**
 * @type {function(number): number}
 */
const double = (x) => x * 2;
"#;

    // Codegen path
    let codegen_result = js2zig_core::native_proto::transpile_js(js, None, None).unwrap();
    println!("=== CODEGEN ===");
    for (i, line) in codegen_result.zig_code.lines().enumerate() {
        println!("{:3}: |{}|", i + 1, line);
    }
    println!();

    // Emitter path
    let alloc = oxc_allocator::Allocator::default();
    let program = parser::parse(&alloc, js);
    let (typedefs, type_annotations, return_types, param_types) = jsdoc::extract_all_jsdoc(js);
    let jsdoc_data = JSDocData { typedefs, type_annotations, return_types, param_types };
    let mut inferrer = TypeInferrer::new();
    inferrer.set_jsdoc_data(jsdoc_data.clone());
    let type_info = inferrer.infer_all(&program, None);

    let mut lowerer = Lowerer::new(type_info, jsdoc_data, None, HashSet::new(), js.to_string());
    let ir_module = lowerer.lower(&program);
    let emitter_output = Emitter::emit_module(&ir_module);
    println!("=== EMITTER ===");
    for (i, line) in emitter_output.lines().enumerate() {
        println!("{:3}: |{}|", i + 1, line);
    }
}
