import sys

with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix 1: Always use 'var' for dynamic access variables (HashMap needs runtime init)
old_kw = '''        let kw = match vd_kind {
            VariableDeclarationKind::Const => "const",
            _ => "var",
        };'''

new_kw = '''        // Dynamic access variables MUST use 'var' (HashMap needs runtime allocator)
        let kw = "var";'''

if old_kw in content:
    content = content.replace(old_kw, new_kw)
    print('Done - fixed kw to always use "var"')
else:
    print('WARNING: could not find kw determination')

# Fix 2: Add 'catch unreachable' to try calls
content = content.replace('self.push(");\n");', 'self.push(" catch unreachable);\n");')

print('Done - added "catch unreachable" to put() calls')

with open('js2rustc/src/codegen.rs', 'w', encoding='utf-8') as f:
    f.write(content)
