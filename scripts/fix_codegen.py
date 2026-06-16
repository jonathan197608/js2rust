with open('js2rustc/src/codegen.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix 1: Remove dynamic_access_vars from new()
content = content.replace(
    '            current_obj_structs: Vec::new(),\n            dynamic_access_vars: HashSet::new(),\n        }\n    }',
    '            current_obj_structs: Vec::new(),\n        }\n    }'
)

# Fix 2: Remove refresh_dynamic_access_vars method
content = content.replace(
    '    /// Refresh dynamic_access_vars from the inferrer (call after inference).\n    fn refresh_dynamic_access_vars(&mut self) {\n        self.dynamic_access_vars = self.inferrer.get_dynamic_access_vars().clone();\n    }\n\n    // ========== Closure pre-scan ==========',
    '    // ========== Closure pre-scan =========='
)

with open('js2rustc/src/codegen.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Done - removed dynamic_access_vars from new() and refresh method')
