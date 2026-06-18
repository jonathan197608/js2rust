// T-CABI-01: Basic i64 export
export function cabiAdd(a, b) { return a + b; }
const test_cabi_add = cabiAdd(3, 5); // => 8

// T-CABI-05: bool export
export function cabiIsPositive(x) { return x > 0; }
const test_cabi_pos = cabiIsPositive(5); // => true
const test_cabi_neg = cabiIsPositive(-3); // => false

// T-CABI-02: String param + return
export function cabiGreet(name) { return "Hello " + name; }
const test_cabi_greet = cabiGreet("World"); // => "Hello World"
