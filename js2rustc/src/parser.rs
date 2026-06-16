use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Parse JS source text.
/// Auto-detects module vs script mode based on presence of `import`/`export` keywords.
pub fn parse<'a>(allocator: &'a Allocator, source: &'a str) -> Program<'a> {
    let is_module = source.contains("import ") || source.contains("export ");
    let source_type = if is_module {
        SourceType::default().with_module(true)
    } else {
        SourceType::default()
    };
    let ret = Parser::new(allocator, source, source_type).parse();

    for err in &ret.errors {
        eprintln!("Parse error: {:?}", err);
    }

    ret.program
}
