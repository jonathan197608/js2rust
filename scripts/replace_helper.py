import sys

with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# Find the start and end of emit_dynamic_access_var_init method
start = None
end = None
brace_count = 0
in_method = False

for i, line in enumerate(lines):
    if 'fn emit_dynamic_access_var_init' in line:
        start = i
        in_method = True
        brace_count = 0
    
    if in_method:
        brace_count += line.count('{') - line.count('}')
        if brace_count == 0 and start is not None and i > start:
            end = i
            break

if start is None:
    print('ERROR: method not found')
    sys.exit(1)

print(f'Found method at lines {start+1}-{end+1}')

# Read the file as a single string
with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find the method and replace it
method_start = content.find('    fn emit_dynamic_access_var_init')
if method_start == -1:
    print('ERROR: method not found via string search')
    sys.exit(1)

# Find the end of the method (matching braces)
brace_count = 0
in_method = False
method_end = method_start

for i in range(method_start, len(content)):
    if content[i] == '{':
        in_method = True
        brace_count += 1
    elif content[i] == '}':
        brace_count -= 1
        if in_method and brace_count == 0:
            method_end = i + 1
            break

print(f'Method spans characters {method_start}-{method_end}')

# New method implementation
new_method = '''    /// Emit HashMap initialization for variables that need dynamic property access.
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
                self.push(".put(\\\"");
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
                        self.push("JsValue{ .string = \\\"");
                        self.push(&lit.value);
                        self.push("\\\" }");
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
                        // Unsupported - store as int 0
                        self.push("JsValue{ .int = 0 }");
                    }
                }
                self.push(");\\n");
            }
        }
    }

'''

# Replace the method
content = content[:method_start] + new_method + content[method_end:]

with open('js2rustc/src/codegen.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Done - replaced emit_dynamic_access_var_init with correct implementation')
