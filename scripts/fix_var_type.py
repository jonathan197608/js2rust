import sys

with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old = '''        self.push(kw);
        self.push(" ");
        self.push(name);
        self.push(" = std.HashMap([]const u8, JsValue).init(std.heap.page_allocator);\\n");'''

new = '''        self.push(kw);
        self.push(" ");
        self.push(name);
        self.push(": std.HashMap([]const u8, JsValue) = std.HashMap([]const u8, JsValue).init(std.heap.page_allocator);\\n");'''

if old in content:
    content = content.replace(old, new)
    print('Done - added type annotation to var declaration')
else:
    print('WARNING: could not find target text')
    idx = content.find('emit_dynamic_access_var_init')
    if idx != -1:
        print(f'Found method at {idx}')
        # Show context
        lines = content[idx:idx+2000].split('\\n')
        for i, line in enumerate(lines):
            if 'HashMap' in line and 'init' in line:
                print(f'Line {i}: {line.strip()}')

with open('js2rustc/src/codegen.rs', 'w', encoding='utf-8') as f:
    f.write(content)
