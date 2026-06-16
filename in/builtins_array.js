// builtins_array.js — Array built-in method tests

function testArrayPush() {
    var arr = [1, 2];
    arr.push(3);
    return arr[2];
}
function testArrayPop() {
    var arr = [10, 20, 30];
    return arr.pop();
}
function testArrayShift() {
    var arr = [10, 20, 30];
    return arr.shift();
}
function testArrayUnshift() {
    var arr = [10, 20];
    arr.unshift(5);
    return arr[0];
}
function testArrayLength() {
    var arr = [1, 2, 3];
    arr.push(4);
    return arr.length;
}

const test_testArrayPush = testArrayPush();
const test_testArrayPop = testArrayPop();
const test_testArrayShift = testArrayShift();
const test_testArrayUnshift = testArrayUnshift();
const test_testArrayLength = testArrayLength();
