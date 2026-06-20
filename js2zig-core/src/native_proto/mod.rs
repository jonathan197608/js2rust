// js2zig-core/src/native_proto/mod.rs
//
// Native-type system codegen module.
// All Codegen impl methods are in codegen.rs.

use oxc_parser::Parser;
use oxc_allocator::Allocator;
use oxc_span::SourceType;

mod codegen;
#[cfg(test)]
mod tests;

/// Transpile a JS string to Zig source (native type system).
pub fn transpile_js(js_source: &str) -> Result<String, String> {
    let alloc = Allocator::default();
    let source_type = SourceType::default();
    let ret = Parser::new(&alloc, js_source, source_type).parse();
    if !ret.errors.is_empty() {
        return Err(format!("Parse errors: {:?}", ret.errors));
    }
    let mut cg = Codegen::new();
    cg.generate(&ret.program);
    Ok(cg.output)
}

/// Shared state for native-type codegen.
pub struct Codegen {
    pub output: String,
    pub indent: usize,
    pub used_names: std::collections::HashSet<String>,
}
