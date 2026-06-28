// test_array_methods.js - Test array methods (push, pop)
export function testArrayMethods() {
    let arr = [1, 2, 3];
    console.log("initial arr.length = " + arr.length);
    
    // Test push
    arr.push(4);
    console.log("after push, arr.length = " + arr.length);
    
    // Test pop
    let last = arr.pop();
    console.log("popped: " + last);
    console.log("after pop, arr.length = " + arr.length);
    
    return 0;
}
