// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 111
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_111.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_111() {

        isNaN(NaN); // true
        isNaN(undefined); // true
        isNaN({}); // true

        isNaN(true); // false
        isNaN(null); // false
        isNaN(37); // false

        // Strings
        isNaN("37"); // false: "37" is converted to the number 37 which is not NaN
        isNaN("37.37"); // false: "37.37" is converted to the number 37.37 which is not NaN
        isNaN("37,5"); // true
        isNaN("123ABC"); // true: Number("123ABC") is NaN
        isNaN(""); // false: the empty string is converted to 0 which is not NaN
        isNaN(" "); // false: a string with spaces is converted to 0 which is not NaN

        // Dates
        isNaN(new Date()); // false; Date objects can be converted to a number (timestamp)
        isNaN(new Date().toString()); // true; the string representation of a Date object cannot be parsed as a number

        // Arrays
        isNaN([]); // false; the primitive representation is "", which coverts to the number 0
        isNaN([1]); // false; the primitive representation is "1"
        isNaN([1, 2]); // true; the primitive representation is "1,2", which cannot be parsed as number
    }
