// generate_expected.js — Run all test functions in Node.js
// and print output in the same format as the Rust main.rs.
// Usage: node generate_expected.js > ../expected_output.txt

import { testExpressions } from "./test_expressions.js";
import { testStatements } from "./test_statements.js";
import { testBuiltins }   from "./test_builtins.js";

console.log("=== MDN JS Reference Tests ===");

console.log("\n=== EXPRESSIONS ===");
testExpressions();

console.log("\n=== STATEMENTS ===");
testStatements();

console.log("\n=== BUILTINS ===");
testBuiltins();

console.log("\n=== All tests done ===");
