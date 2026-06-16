import sys

with open('js2rustc/src/project.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Add jsvalue import after js_array import (two places - single lib and groups)
old = '    out.push_str("const js_array = @import(\\"js_runtime/js_array.zig\\\");\\n");\n    out.push(\'\\n\');'
new = '    out.push_str("const js_array = @import(\\"js_runtime/js_array.zig\\\");\\n");\n    out.push_str("const jsvalue = @import(\\"js_runtime/jsvalue.zig\\\");\\n");\n    out.push(\'\\n\');'

if old in content:
    content = content.replace(old, new)
    print('Done - added jsvalue import to project.rs')
else:
    print('WARNING: could not find target text')
    # Debug: find js_array
    idx = content.find('js_array')
    if idx != -1:
        print(f'Found js_array at {idx}: {content[idx:idx+100]}')

with open('js2rustc/src/project.rs', 'w', encoding='utf-8') as f:
    f.write(content)
