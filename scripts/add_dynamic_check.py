import sys

with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find the ObjectExpression arm and add dynamic access check
old_text = '''                    Expression::ObjectExpression(obj) if self.in_top_level => {
                        let obj_type = self.inferrer.infer_expr(init);'''

new_text = '''                    Expression::ObjectExpression(obj) if self.in_top_level => {
                        // Check if this variable needs dynamic access (HashMap instead of struct)
                        if self.inferrer.get_dynamic_access_vars().contains(name) {
                            self.emit_dynamic_access_var_init(name, kw, obj);
                            continue;
                        }
                        let obj_type = self.inferrer.infer_expr(init);'''

if old_text in content:
    content = content.replace(old_text, new_text)
    print('Done - added dynamic access check to emit_var_decl')
else:
    print('WARNING: could not find ObjectExpression arm')
    # Debug: find similar text
    idx = content.find('Expression::ObjectExpression(obj)')
    if idx != -1:
        print(f'Found at index {idx}')
        print(f'Context: {content[idx:idx+200]}')

with open('js2rustc/src/codegen.rs', 'w', encoding='utf-8') as f:
    f.write(content)
