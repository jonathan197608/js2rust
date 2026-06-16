import sys

with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find and replace the numeric literal handling in emit_dynamic_access_var_init
old_code = '''                    Expression::NumericLiteral(lit) => {
                        if lit.value.fract() != 0.0 {
                            self.push("JsValue{ .float = ");
                            self.push(&lit.raw.as_ref().unwrap_or(&lit.value.to_string()));
                            self.push(" }");
                        } else {
                            self.push("JsValue{ .int = ");
                            self.push(&lit.raw.as_ref().unwrap_or(&lit.value.to_string()));
                            self.push(" }");
                        }
                    }'''

new_code = '''                    Expression::NumericLiteral(lit) => {
                        let val_str = lit.value.to_string();
                        if lit.value.fract() != 0.0 {
                            self.push("JsValue{ .float = ");
                            self.push(&val_str);
                            self.push(" }");
                        } else {
                            self.push("JsValue{ .int = ");
                            self.push(&val_str);
                            self.push(" }");
                        }
                    }'''

if old_code in content:
    content = content.replace(old_code, new_code)
    print('Done - fixed NumericLiteral handling')
else:
    print('WARNING: could not find NumericLiteral handling to replace')
    # Debug: print what we're looking for
    idx = content.find('NumericLiteral')
    if idx != -1:
        print(f'Found NumericLiteral at index {idx}')
        print(f'Context: {content[idx:idx+200]}')

with open('js2rustc/src/codegen.rs', 'w', encoding='utf-8') as f:
    f.write(content)
