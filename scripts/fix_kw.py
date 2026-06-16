import sys

with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix 1: Change emit_dynamic_access_var_init to take vd_kind
old_sig = '''    fn emit_dynamic_access_var_init(&mut self, name: &str, kw: &str, obj: &ObjectExpression) {'''
new_sig = '''    fn emit_dynamic_access_var_init(&mut self, name: &str, vd_kind: &VariableDeclarationKind, obj: &ObjectExpression) {'''

if old_sig in content:
    content = content.replace(old_sig, new_sig)
    print('Done - fixed method signature')
else:
    print('WARNING: could not find old signature')
    idx = content.find('fn emit_dynamic_access_var_init')
    if idx != -1:
        print(f'Found method at index {idx}')
        print(f'Context: {content[idx:idx+200]}')

# Fix 2: Inside the method, replace self.push(kw) with proper kw determination
old_kw = '''        self.push(kw);
        self.push(" ");
        self.push(name);
        self.push(" = std.HashMap([]const u8, JsValue).init(std.heap.page_allocator);\\n");'''

new_kw = '''        const kw = match vd_kind {
            VariableDeclarationKind::Const => "const",
            _ => "var",
        };
        self.push(kw);
        self.push(" ");
        self.push(name);
        self.push(" = std.HashMap([]const u8, JsValue).init(std.heap.page_allocator);\\n");'''

if old_kw in content:
    content = content.replace(old_kw, new_kw)
    print('Done - fixed kw usage inside method')
else:
    print('WARNING: could not find old kw usage')

# Fix 3: Update call site to pass &vd.kind instead of kw
old_call = '''self.emit_dynamic_access_var_init(name, kw, obj);'''
new_call = '''self.emit_dynamic_access_var_init(name, &vd.kind, obj);'''

if old_call in content:
    content = content.replace(old_call, new_call)
    print('Done - fixed call site')
else:
    print('WARNING: could not find old call site')

with open('js2rustc/src/codegen.rs', 'w', encoding='utf-8') as f:
    f.write(content)
