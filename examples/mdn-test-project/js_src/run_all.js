// Run all MDN test functions.
// Usage:  node run_all.js  > expected_output.json

const _captured = [];
const _orig = console.log;
console.log = function(...args) {
    _captured.push(args.length === 1 ? String(args[0]) : args.map(String).join(' '));
};

const results = {};

try {
    const { testExpressions } = require('./test_expressions.js');
    _captured.length = 0;
    testExpressions();
    results.expressions = [..._captured];
} catch(e) { results.expressions = ['[FATAL] ' + e.message]; }

try {
    const { testStatements } = require('./test_statements.js');
    _captured.length = 0;
    testStatements();
    results.statements = [..._captured];
} catch(e) { results.statements = ['[FATAL] ' + e.message]; }

try {
    const { testBuiltins } = require('./test_builtins.js');
    _captured.length = 0;
    testBuiltins();
    results.builtins = [..._captured];
} catch(e) { results.builtins = ['[FATAL] ' + e.message]; }

console.log = _orig;
console.log(JSON.stringify(results, null, 2));