// T-BUILTIN-11: Array — static array basics
function testArrLen() {
    const arr = [1, 2, 3, 4, 5];
    return arr.length;
}
const test_arr_len = testArrLen(); // => 5

function testArrIncludes() {
    const arr = [10, 20, 30];
    return arr.includes(20);
}
const test_arr_includes = testArrIncludes(); // => true

function testArrIndexOf() {
    const arr = [10, 20, 30];
    return arr.indexOf(20);
}
const test_arr_indexof = testArrIndexOf(); // => 1
