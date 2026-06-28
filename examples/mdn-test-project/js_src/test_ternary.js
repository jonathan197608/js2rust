export function testTernaryString() {
    let x = 10;
    let y = 20;
    
    // Test 1: Simple ternary with strings
    let a = true ? "hello" : "world";
    
    // Test 2: Ternary + string concat
    let b = "result: " + (x > 5 ? "big" : "small");
    
    // Test 3: Ternary with numbers + string concat
    let c = "value: " + (x > 5 ? x : y);
    
    return 0;
}
