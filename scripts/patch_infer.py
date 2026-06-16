import re

with open('js2rustc/src/infer.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# New methods to insert before the LAST } (closing impl TypeInferrer)
new_methods = '''    /// Returns the set of variable names accessed with a dynamic (non-literal) key.
    /// These variables must use HashMap instead of a generated struct.
    pub fn get_dynamic_access_vars(&self) -> &HashSet<String> {
        &self.dynamic_access_vars
    }

    /// Pre-pass: walk the entire program and populate `dynamic_access_vars`.
    fn detect_dynamic_access(&mut self, program: &Program) {
        for stmt in &program.body {
            self.detect_dynamic_access_stmt(stmt);
        }
    }

    fn detect_dynamic_access_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(v) => {
                for decl in &v.declarations {
                    if let Some(init) = &decl.init {
                        self.detect_dynamic_access_expr(init);
                    }
                }
            }
            Statement::ExpressionStatement(e) => {
                self.detect_dynamic_access_expr(&e.expression);
            }
            Statement::ReturnStatement(r) => {
                if let Some(arg) = &r.argument {
                    self.detect_dynamic_access_expr(arg);
                }
            }
            Statement::IfStatement(i) => {
                self.detect_dynamic_access_expr(&i.test);
                self.detect_dynamic_access_stmt(&i.consequent);
                if let Some(alt) = &i.alternate {
                    self.detect_dynamic_access_stmt(alt);
                }
            }
            Statement::ForStatement(f) => {
                if let Some(init) = &f.init {
                    if let ForStatementInit::VariableDeclaration(v) = init {
                        for decl in &v.declarations {
                            if let Some(init) = &decl.init {
                                self.detect_dynamic_access_expr(init);
                            }
                        }
                    }
                }
                if let Some(test) = &f.test {
                    self.detect_dynamic_access_expr(test);
                }
                if let Some(update) = &f.update {
                    self.detect_dynamic_access_expr(update);
                }
                self.detect_dynamic_access_stmt(&f.body);
            }
            Statement::WhileStatement(w) => {
                self.detect_dynamic_access_expr(&w.test);
                self.detect_dynamic_access_stmt(&w.body);
            }
            Statement::DoWhileStatement(d) => {
                self.detect_dynamic_access_stmt(&d.body);
                self.detect_dynamic_access_expr(&d.test);
            }
            Statement::BlockStatement(b) => {
                for s in &b.body {
                    self.detect_dynamic_access_stmt(s);
                }
            }
            Statement::SwitchStatement(s) => {
                self.detect_dynamic_access_expr(&s.discriminant);
                for case in &s.cases {
                    if let Some(test) = &case.test {
                        self.detect_dynamic_access_expr(test);
                    }
                    for s in &case.consequent {
                        self.detect_dynamic_access_stmt(s);
                    }
                }
            }
            Statement::FunctionDeclaration(f) => {
                if let Some(body) = &f.body {
                    for s in &body.statements {
                        self.detect_dynamic_access_stmt(s);
                    }
                }
            }
            _ => {}
        }
    }

    fn detect_dynamic_access_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::ComputedMemberExpression(mem) => {
                // If key is NOT a string literal, mark object as dynamic access
                if !matches!(&mem.expression, Expression::StringLiteral(_)) {
                    if let Expression::Identifier(id) = &*mem.object {
                        self.dynamic_access_vars.insert(id.name.to_string());
                    }
                }
                // Recurse into object and key
                self.detect_dynamic_access_expr(&mem.object);
                self.detect_dynamic_access_expr(&mem.expression);
            }
            Expression::MemberExpression(mem) => {
                self.detect_dynamic_access_expr(&mem.object);
            }
            Expression::CallExpression(call) => {
                self.detect_dynamic_access_expr(&call.callee);
                for arg in &call.arguments {
                    if let Argument::Expression(e) = arg {
                        self.detect_dynamic_access_expr(e);
                    }
                }
            }
            Expression::BinaryExpression(bin) => {
                self.detect_dynamic_access_expr(&bin.left);
                self.detect_dynamic_access_expr(&bin.right);
            }
            Expression::UnaryExpression(u) => {
                self.detect_dynamic_access_expr(&u.argument);
            }
            Expression::AssignmentExpression(a) => {
                self.detect_dynamic_access_expr(&a.left);
                self.detect_dynamic_access_expr(&a.right);
            }
            Expression::ConditionalExpression(c) => {
                self.detect_dynamic_access_expr(&c.test);
                self.detect_dynamic_access_expr(&c.consequent);
                self.detect_dynamic_access_expr(&c.alternate);
            }
            Expression::ArrayExpression(a) => {
                for elem in &a.elements {
                    if let Some(Argument::Expression(e)) = elem {
                        self.detect_dynamic_access_expr(e);
                    }
                }
            }
            Expression::ObjectExpression(o) => {
                for prop in &o.properties {
                    if let Expression::ObjectExpressionProperty(p) = prop {
                        if let Some(v) = &p.value {
                            self.detect_dynamic_access_expr(v);
                        }
                    }
                }
            }
            Expression::ParenthesizedExpression(p) => {
                self.detect_dynamic_access_expr(&p.expression);
            }
            Expression::SequenceExpression(s) => {
                for e in &s.expressions {
                    self.detect_dynamic_access_expr(e);
                }
            }
            Expression::AwaitExpression(a) => {
                self.detect_dynamic_access_expr(&a.argument);
            }
            Expression::ArrowFunctionExpression(arrow) => {
                for s in &arrow.body.statements {
                    self.detect_dynamic_access_stmt(s);
                }
            }
            Expression::FunctionExpression(fe) => {
                if let Some(body) = &fe.body {
                    for s in &body.statements {
                        self.detect_dynamic_access_stmt(s);
                    }
                }
            }
            _ => {}
        }
    }

'''

# Insert new_methods before the LAST } (closing impl TypeInferrer)
# Find position of last } that is at the start of a line (with only whitespace before it)
lines = content.split('\n')

# The last line is '}' (closing impl)
# Insert new_methods before the last line
new_lines = lines[:-1] + [new_methods.rstrip('\n')] + [lines[-1]]
new_content = '\n'.join(new_lines)

with open('js2rustc/src/infer.rs', 'w', encoding='utf-8') as f:
    f.write(new_content)

print('Done: inserted detect_dynamic_access methods')
