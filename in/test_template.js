// Template literal tests

// T-TPL-01: simple template (no interpolation) → plain string literal
function testSimpleTemplate() {
    return `hello`;
}
const test_simple_tpl = testSimpleTemplate(); // => "hello"

// T-TPL-02: template with integer interpolation → std.fmt.allocPrint
function testTemplateInt() {
    const x = 42;
    return `n=${x}`;
}
const test_tpl_int = testTemplateInt(); // => "n=42"

// T-TPL-03: template with arithmetic expression
function testTemplateExpr() {
    const a = 3;
    const b = 5;
    return `sum=${a + b}`;
}
const test_tpl_expr = testTemplateExpr(); // => "sum=8"

// T-TPL-04: template with multiple interpolations
function testTemplateMulti() {
    const x = 10;
    const y = 20;
    return `${x}+${y}=${x + y}`;
}
const test_tpl_multi = testTemplateMulti(); // => "10+20=30"

// T-TPL-05: template as concat (adjacent text + variable)
function testTemplateConcat() {
    const prefix = 99;
    return `id_${prefix}`;
}
const test_tpl_concat = testTemplateConcat(); // => "id_99"
