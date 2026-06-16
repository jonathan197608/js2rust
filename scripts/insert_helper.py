import sys

with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find the line where emit_var_decl starts
marker = '    fn emit_var_decl(&mut self, vd: &VariableDeclaration) {'
idx = content.find(marker)
if idx == -1:
    print('ERROR: could not find emit_var_decl')
    sys.exit(1)

# Insert helper method before emit_var_decl
helper = '''    /// Emit HashMap initialization for variables that need dynamic property access.
    /// Generates:
    ///   var name = std.StringArrayHashMap(JsValue).init(std.heap.page_allocator);
    ///   try name.put("field", JsValue{ .tag = value });
    fn emit_dynamic_access_var_init(&mut self, name: &str, kw: &str, obj: &ObjectExpression) {
        // Import JsValue and HashMap type
        self.emit_indent();
        self.push(kw);
        self.push(" ");
        self.push(name);
        self.push(" = std.StringArrayHashMap(JsValue).init(std.heap.page_allocator);\\n");
        
        for prop in &obj.properties {
            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                let field_name = match &p.key {
                    oxc_ast::ast::PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                    oxc_ast::ast::PropertyKey::StringLiteral(s) => s.value.to_string(),
                    _ => continue,
                };
                self.emit_indent();
                self.push("try ");
                self.push(name);
                self.push(".put(\\"");
                self.push(&field_name);
                self.push("\\", ");
                self.emit_expr(&p.value);
                self.push(");\\n");
            }
        }
    }

'''

content = content[:idx] + helper + content[idx:]

with open('js2rustc/src/codegen.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Done - inserted emit_dynamic_access_var_init before emit_var_decl')
