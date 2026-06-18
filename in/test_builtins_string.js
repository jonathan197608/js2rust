// T-BUILTIN-08: String methods — search/query
// Use inline string constants to avoid parameter-dependent type inference
function testIncludes() {
    const s = "hello world";
    return s.includes("world");
}
const test_includes_t = testIncludes(); // => true

function testIncludesFalse() {
    const s = "hello world";
    return s.includes("xyz");
}
const test_includes_f = testIncludesFalse(); // => false

function testIndexOf() {
    const s = "hello world";
    return s.indexOf("world");
}
const test_indexof_found = testIndexOf(); // => 6

function testIndexOfMiss() {
    const s = "hello world";
    return s.indexOf("xyz");
}
const test_indexof_miss = testIndexOfMiss(); // => -1

function testStartsWithTrue() {
    const s = "hello world";
    return s.startsWith("hello");
}
const test_starts_t = testStartsWithTrue(); // => true

function testStartsWithFalse() {
    const s = "hello world";
    return s.startsWith("world");
}
const test_starts_f = testStartsWithFalse(); // => false

function testEndsWithTrue() {
    const s = "hello world";
    return s.endsWith("world");
}
const test_ends_t = testEndsWithTrue(); // => true

function testEndsWithFalse() {
    const s = "hello world";
    return s.endsWith("hello");
}
const test_ends_f = testEndsWithFalse(); // => false

// T-BUILTIN-10: String.trim (no allocator)
function testTrim() {
    const s = "  hello  ";
    return s.trim();
}
const test_trim = testTrim(); // => "hello"

// String.length
function testStrLen() {
    const s = "hello";
    return s.length;
}
const test_strlen = testStrLen(); // => 5

// T-BUILTIN-08b: String toUpperCase / toLowerCase (needs allocator)
function testToUpper() {
    const s = "hello";
    return s.toUpperCase();
}
const test_upper = testToUpper(); // => "HELLO"

function testToLower() {
    const s = "HELLO";
    return s.toLowerCase();
}
const test_lower = testToLower(); // => "hello"
