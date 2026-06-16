import sys

with open('js2rustc/src/infer.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# Find the line number where "fn detect_dynamic_access_expr" starts (0-indexed)
insert_idx = None
for i, line in enumerate(lines):
    if 'fn detect_dynamic_access_expr' in line and '(&mut self' in line:
        insert_idx = i
        break

if insert_idx is None:
    print('ERROR: could not find detect_dynamic_access_expr')
    sys.exit(1)

new_method = '''    fn detect_dynamic_access_assign_target(&mut self, target: &AssignmentTarget) {
        match target {
            AssignmentTarget::SimpleAssignmentTarget(simple) => {
                match simple {
                    SimpleAssignmentTarget::MemberExpression(mem) => {
                        match mem {
                            MemberExpression::ComputedMemberExpression(cm) => {
                                if !matches!(&cm.expression, Expression::StringLiteral(_)) {
                                    if let Expression::Identifier(id) = &cm.object {
                                        self.dynamic_access_vars.insert(id.name.to_string());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

'''

# Insert new_method before insert_idx
lines.insert(insert_idx, new_method)

with open('js2rustc/src/infer.rs', 'w', encoding='utf-8') as f:
    f.writelines(lines)

print(f'Done - inserted detect_dynamic_access_assign_target before line {insert_idx + 1}')
