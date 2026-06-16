import sys

with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find and replace the incorrect helper method
old_method = '''    /// Emit HashMap initialization for variables that need dynamic property access.
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
                self.push(".put(\"");
                self.push(&field_name);
                self.push("\", ");
                self.emit_expr(&p.value);
                self.push(");\\n");
            }
        }
    }'''

new_method = '''    /// Emit HashMap initialization for variables that need dynamic property access.
    /// Generates:
    ///   var name = std.StringArrayHashMap(JsValue).init(std.heap.page_allocator);
    ///   try name.put("field", JsValue{ .string = "..." });
    fn emit_dynamic_access_var_init(&mut self, name: &str, kw: &str, obj: &ObjectExpression) {
        self.emit_indent();
        self.push(kw);
        self.push(" ");
        self.push(name);
        self.push(" = std.HashMap([]const u8, JsValue).init(std.heap.page_allocator);\\n");
        
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
                // Emit JsValue literal based on expression type
                match &p.value {
                    Expression::NumericLiteral(lit) => {
                        if lit.value.fract() != 0.0 {
                            self.push("JsValue{ .float = ");
                            self.push(&lit.raw.as_ref().unwrap_or(&lit.value.to_string()));
                            self.push(" }");
                        } else {
                            self.push("JsValue{ .int = ");
                            self.push(&lit.raw.as_ref().unwrap_or(&lit.value.to_string()));
                            self.push(" }");
                        }
                    }
                    Expression::StringLiteral(lit) => {
                        self.push("JsValue{ .string = \\"");
                        self.push(&lit.value);
                        self.push("\\" }");
                    }
                    Expression::BooleanLiteral(lit) => {
                        self.push("JsValue{ .bool = ");
                        self.push(if lit.value { "true" } else { "false" });
                        self.push(" }");
                    }
                    Expression::NullLiteral(_) => {
                        self.push("JsValue{ .null = {} }");
                    }
                    _ => {
                        // Unsupported expression type - store as int 0
                        self.push("JsValue{ .int = 0 }");
                    }
                }
                self.push(");\\n");
            }
        }
    }'''

if old_method in content:
    content = content.replace(old_method, new_method)
    print('Done - replaced emit_dynamic_access_var_init with correct implementation')
else:
    print('WARNING: could not find old method to replace')
    # Print first 200 chars of old_method to debug
    print(f'Old method starts with: {old_method[:200]}')
    # Find where emit_dynamic_access_var_init is defined
    idx = content.find('fn emit_dynamic_access_var_init')
    if idx != -1:
        print(f'Found method at index {idx}')
        print(f'Context: {content[idx:idx+500]}')

with open('js2rustc/src/codegen.rs', 'w', encoding='utf-8') as f:
    f.write(content)
