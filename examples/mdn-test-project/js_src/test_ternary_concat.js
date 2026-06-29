// test_ternary_concat.js - Minimal test for ternary + string concat
export function testTernaryConcat() {
    let x = 10;
    
    // Test: ternary with strings + string concat
    let result = "value: " + (x > 5 ? "big" : "small");
    console.log(result);
    return 0;
}
