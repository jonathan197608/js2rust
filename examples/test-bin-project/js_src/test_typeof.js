// test_typeof.js - Test typeof operator behavior
// This file tests the transpiled behavior of typeof

function testTypeof() {
    // JS typeof returns: "number", "string", "boolean", "undefined", "object", "function"
    // Zig @typeName(@TypeOf(x)) returns: type name like "i64", "[]const u8", "bool"
    
    const n = 42;
    const s = "hello";
    const b = true;
    
    // These will return Zig type names, not JS type names
    const typeN = typeof n;  // JS: "number", Zig: "i64" (probably)
    const typeS = typeof s;  // JS: "string", Zig: "[]const u8" (probably)
    const typeB = typeof b;  // JS: "boolean", Zig: "bool" (probably)
    
    return typeN + " " + typeS + " " + typeB;
}

if (typeof exports !== 'undefined') {
    exports.testTypeof = testTypeof;
}
