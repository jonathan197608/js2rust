// Run all 3 test functions and capture console output.
// Usage:  node run_all.js

const _captured = [];
const _origLog   = console.log;
const _origError = console.error;
const _origWarn  = console.warn;
console.log = function(...args) {
    _captured.push(args.length === 1 ? args[0] : args);
};
console.error = function(...args) {
    _captured.push('[error] ' + (args.length === 1 ? args[0] : args.join(' ')));
};
console.warn = function(...args) {
    _captured.push('[warn] ' + (args.length === 1 ? args[0] : args.join(' ')));
};

import { testExpressions } from "./test_expressions.js";
import { testStatements } from "./test_statements.js";
import { testBuiltins }   from "./test_builtins.js";

const results = {};

_captured.length = 0;
testExpressions();
results.expressions = [..._captured];

_captured.length = 0;
testStatements();
results.statements = [..._captured];

_captured.length = 0;
testBuiltins();
results.builtins = [..._captured];

console.log   = _origLog;
console.error = _origError;
console.warn  = _origWarn;
console.log(JSON.stringify(results, null, 2));
