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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_expression() {
        let alloc = Allocator::default();
        let program = parse(&alloc, "1 + 2;");
        assert!(!program.body.is_empty());
    }

    #[test]
    fn parse_function_declaration() {
        let alloc = Allocator::default();
        let program = parse(&alloc, "function add(a, b) { return a + b; }");
        assert!(!program.body.is_empty());
    }

    #[test]
    fn parse_module_with_import_export() {
        let alloc = Allocator::default();
        let program = parse(
            &alloc,
            "import { x } from './m.js'; export function f() { return x; }",
        );
        assert!(!program.body.is_empty());
    }

    #[test]
    fn parse_async_function() {
        let alloc = Allocator::default();
        let program = parse(&alloc, "async function fetch() { return 1; }");
        assert!(!program.body.is_empty());
    }

    #[test]
    fn parse_class_declaration() {
        let alloc = Allocator::default();
        let program = parse(&alloc, "class Point { constructor(x,y) { this.x=x; this.y=y; } }");
        assert!(!program.body.is_empty());
    }

    #[test]
    fn parse_arrow_function() {
        let alloc = Allocator::default();
        let program = parse(&alloc, "const add = (a, b) => a + b;");
        assert!(!program.body.is_empty());
    }

    #[test]
    fn parse_empty_program() {
        let alloc = Allocator::default();
        let program = parse(&alloc, "");
        assert!(program.body.is_empty());
    }
}
