// test_nested_function.js - Test nested function return type inference
export function testNestedFunction() {
    function add(a, b) {
        return a + b;
    }
    let result = add(1, 2);
    console.log("1+2 = " + result);
    return 0;
}
