// T-CLS-03: closure — variable assignment (simplest form)
function createMultiplier(factor) {
    const mul = (x) => x * factor;
    return mul(5);
}
const test_closure_var = createMultiplier(3); // => 15

// T-CLS-01: basic closure — capture single variable
function makeAdder(x) {
    const add = (y) => x + y;
    return add(5);
}
const test_adder = makeAdder(10); // => 15

// T-CLS-05: nested closure
function outer(a) {
    const inner = (b) => a * b;
    return inner(4);
}
const test_nested = outer(3); // => 12
