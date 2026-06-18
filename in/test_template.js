// Template literal tests — dynamic arrays, object properties, multi-line

// T-TPL-01: Dynamic array element interpolation
// ArrayList push + index access inside template
function testTplDynArr() {
    const arr = [];
    arr.push(10);
    arr.push(20);
    arr.push(30);
    return `first=${arr[0]},last=${arr[2]}`;
}
const test_tpl_dyn_arr = testTplDynArr(); // => "first=10,last=30"

// T-TPL-02: Object (struct) property interpolation
function testTplObjProp() {
    const point = { x: 10, y: 20 };
    return `(${point.x},${point.y})`;
}
const test_tpl_obj_prop = testTplObjProp(); // => "(10,20)"

// T-TPL-03: Multi-line template (no interpolation)
function testTplMultiLine() {
    return `line1
line2
line3`;
}
const test_tpl_multi_line = testTplMultiLine(); // => "line1\nline2\nline3"

// T-TPL-04: Multi-line template with interpolation
function testTplMultiLineExpr() {
    const a = 100;
    const b = 200;
    return `a=${a}
b=${b}
sum=${a + b}`;
}
const test_tpl_multi_expr = testTplMultiLineExpr(); // => "a=100\nb=200\nsum=300"

// T-TPL-05: Array element arithmetic inside template
function testTplArrMath() {
    const arr = [];
    arr.push(3);
    arr.push(7);
    return `sum=${arr[0] + arr[1]}`;
}
const test_tpl_arr_math = testTplArrMath(); // => "sum=10"

// T-TPL-06: Object property arithmetic inside template
function testTplObjMath() {
    const rect = { w: 5, h: 8 };
    return `area=${rect.w * rect.h}`;
}
const test_tpl_obj_math = testTplObjMath(); // => "area=40"

// T-TPL-07: Combined — array + object + expression in one template
function testTplCombined() {
    const vals = [];
    vals.push(10);
    vals.push(20);
    const cfg = { factor: 3 };
    return `a=${vals[0]},b=${vals[1]},sum=${vals[0] + vals[1]},f=${cfg.factor}`;
}
const test_tpl_combined = testTplCombined(); // => "a=10,b=20,sum=30,f=3"
