import sys

with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix: Replace 'try ' with '' and add ' catch unreachable' before ';\n'
# The current code generates:
#   try person.put("name", JsValue{ .string = "Alice" });
# Should be:
#   person.put("name", JsValue{ .string = "Alice" }) catch unreachable;

old = 'self.push("try ");'
new = 'self.push("');'  # Remove 'try '

if old in content:
    content = content.replace(old, new)
    print('Done - removed "try " prefix')
else:
    print('WARNING: could not find "try " prefix')

# Now add ' catch unreachable' before ');\n' for put() calls
# Actually, the put() calls end with ');\n' - need to change to ') catch unreachable;\n'
old2 = 'self.push(");\n");'
new2 = 'self.push(" catch unreachable);\n");'

if old2 in content:
    content = content.replace(old2, new2)
    print('Done - added "catch unreachable" to put() calls')
else:
    print('WARNING: could not find ");\\n" to replace')

with open('js2rustc/src/codegen.rs', 'w', encoding='utf-8') as f:
    f.write(content)
