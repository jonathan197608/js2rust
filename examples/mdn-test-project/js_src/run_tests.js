// run_tests.js — Run MDN test functions in Node.js and print output
import { testExpressions, testStatements, testBuiltins } from './test_expressions.js';
import { testStatements } from './test_statements.js';
import { testBuiltins } from './test_builtins.js';

console.log("=== EXPRESSIONS ===");
testExpressions();

console.log("\n=== STATEMENTS ===");
testStatements();

console.log("\n=== BUILTINS ===");
testBuiltins();

console.log("\n=== All tests done ===");
