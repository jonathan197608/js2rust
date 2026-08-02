#!/usr/bin/env python3
"""
test262 scanner: transforms test262 test files into js2rust-compatible format.

Usage:
    python scan_test262.py <test262_dir> [--category <path>] [--max <N>]

Scans test262 .js files, applies harness transformations, and generates:
  - js_src/*.js  (transformed test files)
  - js2rust.toml (project config)
  - src/main.rs  (test runner with auto-generated dispatch)

Transformations:
  assert.sameValue(a, b, c)      -> assert_same_value(a, b, c)
  assert.notSameValue(a, b, c)   -> assert_not_same_value(a, b, c)
  throw new Test262Error(msg)    -> host_assert_fail(msg)
  assert.throws(...)             -> (skipped, marked as SKIP)

Skip filters (unsupported features):
  eval, $262, async, await, import, generator, BigInt (n suffix),
  Symbol, Proxy, Reflect, WeakMap, WeakSet, SharedArrayBuffer
"""

import argparse
import os
import re
import sys
from pathlib import Path

# --- Harness preamble (non-exported functions get anytype params) ---
HARNESS = """\
// --- test262 harness (non-exported, anytype params per Rule 7) ---
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}
function assert_not_same_value(actual, expected, message) {
    if (actual === expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.notSameValue failed"); };
    }
}
"""

# --- Skip patterns: tests using these features are skipped ---
SKIP_PATTERNS = [
    r'\$DONOTEVALUATE',      # negative parse/early-error tests
    r'\$MAX_ITERATIONS',     # test262 harness variable
    r'\beval\s*\(',          # eval()
    r'\$262',                # $262 agent
    r'\basync\s+',           # async functions
    r'\bawait\s+',           # await
    r'\bimport\s+',          # import statements (dynamic/static)
    r'\byield\b',            # generators
    r'\bfunction\s*\*',      # generator functions
    r'\bSymbol\b',           # Symbol
    r'\bProxy\b',            # Proxy
    r'\bReflect\b',          # Reflect
    r'\bWeakMap\b',          # WeakMap
    r'\bWeakSet\b',          # WeakSet
    r'\bSharedArrayBuffer\b', # SharedArrayBuffer
    r'\bBigInt\b',           # BigInt constructor
    r'\b\d+n\b',             # BigInt literal (decimal, e.g. 123n)
    r'\b0x[0-9a-fA-F]+n\b',  # BigInt literal (hex, e.g. 0xFFn)
    r'\binstanceof\b',       # instanceof (runtime type checking)
    r'\bassert\.throws\b',   # assert.throws (TODO: convert to try/catch)
    r'\bassert\.doesNotThrow\b', # assert.doesNotThrow
    r'\bassert\.sameValue\s*\(\s*undefined', # some edge cases
    r'\bassert\s*\(',        # bare assert() calls (not assert.something)
    # Destructuring, rest/spread — not supported by transpiler
    r'\.\.\.',               # rest/spread operator
    r'\b(const|let|var)\s*\[',  # array destructuring pattern
    r'\b(const|let|var)\s*\{',  # object destructuring pattern
    r'\{[^}]*\.\.\.',         # object rest destructuring
    r'\btry\s*\{',           # try/catch blocks (transpiler limitation)
    r'\bthis\b',             # this keyword (transpiler limitation)
    # Division by zero — Zig treats this as illegal behavior
    r'\b[+-]?0\s*/\s*[+-]?0\b',  # 0/0, +0/+0, -0/-0 etc. (NaN in JS, illegal in Zig)
    # Unsupported built-in globals
    r'\bparseInt\b',         # global parseInt
    r'\bRegExp\b',           # global RegExp
    r'\bDate\b',             # global Date
    # Unsupported methods / globals
    r'\bArray\.isArray\b',   # Array.isArray
    # new expressions (after Test262Error transform, any remaining 'new' is unsupported)
    r'\bnew\s+(?!Test262Error\b)\w+',  # new AnyConstructor (except Test262Error, already transformed)
    # Assignment as expression: (x = expr) used as operand
    # Matches = inside parens that isn't ==, ===, =>, !=, !==
    r'\([^)]*\w+\s*=(?![=>])\s*[^=)]',
    # Object literals and anonymous functions (ToPrimitive coercion not supported)
    r'\{\s*\}',              # empty object literal {}
    r'\bfunction\s*\([^)]*\)\s*\{',  # anonymous function expression
    r'\b\.valueOf\b',        # valueOf method
    r'\b\.toString\s*\(',    # toString() method call
    r'\bdelete\s+',          # delete operator
    r'\bvoid\s+',            # void operator
    r'\bin\s\s',             # `in` operator (with space to avoid matching 'in' in words)
]

# Compile skip patterns
SKIP_RE = re.compile('|'.join(SKIP_PATTERNS))

# --- Transformation rules ---
def transform_content(content: str) -> str:
    """Apply text transformations to test262 content."""
    # Strip frontmatter /*--- ... ---*/
    content = re.sub(r'/\*---.*?---\*/', '', content, flags=re.DOTALL)

    # Transform assert.sameValue -> assert_same_value
    content = re.sub(r'\bassert\.sameValue\s*\(', 'assert_same_value(', content)

    # Transform assert.notSameValue -> assert_not_same_value
    content = re.sub(r'\bassert\.notSameValue\s*\(', 'assert_not_same_value(', content)

    # Transform: throw new Test262Error(msg) -> host_assert_fail(msg)
    # Handle multi-line and single-line
    content = re.sub(
        r'throw\s+new\s+Test262Error\s*\(',
        'host_assert_fail(',
        content
    )

    # Transform: new Test262Error(msg) used standalone (not in throw)
    # e.g., var err = new Test262Error("msg"); throw err;
    content = re.sub(r'new\s+Test262Error\s*\(', 'host_assert_fail(', content)

    # Fix 2-arg assert calls: add default message "" as 3rd argument
    content = fix_assert_args(content)

    return content


def fix_assert_args(content: str) -> str:
    """Add default "" message to 2-arg assert_same_value/not_same_value calls.
    
    test262's assert.sameValue can be called with 2 or 3 args:
      assert.sameValue(a, b)         -> assert_same_value(a, b, "")
      assert.sameValue(a, b, "msg")  -> assert_same_value(a, b, "msg")
    
    The Zig harness uses anytype params, so all 3 args must be present.
    """
    for func_name in ('assert_same_value', 'assert_not_same_value'):
        content = _add_default_third_arg(content, func_name + '(', '""')
    return content


def _add_default_third_arg(content: str, prefix: str, default_arg: str) -> str:
    """For calls starting with `prefix` that have exactly 2 args, add default_arg as 3rd."""
    result = []
    i = 0
    prefix_len = len(prefix)
    while i < len(content):
        idx = content.find(prefix, i)
        if idx == -1:
            result.append(content[i:])
            break
        # Append everything before the match
        result.append(content[i:idx])
        
        # Find matching closing paren starting from after the prefix
        start = idx + prefix_len
        depth = 1  # we're already inside the first (
        comma_count = 0
        j = start
        while j < len(content) and depth > 0:
            ch = content[j]
            if ch == '(':
                depth += 1
            elif ch == ')':
                depth -= 1
                if depth == 0:
                    break
            elif ch == ',' and depth == 1:
                comma_count += 1
            elif ch == '"' or ch == "'":
                # Skip string literal
                quote = ch
                j += 1
                while j < len(content) and content[j] != quote:
                    if content[j] == '\\':
                        j += 1
                    j += 1
            elif ch == '`':
                # Skip template literal
                j += 1
                while j < len(content) and content[j] != '`':
                    if content[j] == '\\':
                        j += 1
                    j += 1
            j += 1
        
        if depth == 0 and comma_count == 1:
            # 2-arg call: insert default_arg before closing )
            result.append(prefix)
            result.append(content[start:j])
            result.append(', ' + default_arg)
            result.append(')')
        else:
            # 3+ arg call or unparseable: keep as-is
            result.append(prefix)
            if depth == 0:
                result.append(content[start:j + 1])
            else:
                result.append(content[start:])
                break
        
        i = j + 1
    return ''.join(result)


def strip_strings_and_comments(content: str) -> str:
    """Remove string literals and comments from content for pattern matching.
    
    This prevents false positives when skip patterns match text inside strings.
    Replaces strings with empty string placeholders to preserve structure.
    """
    # Remove block comments /* ... */
    result = re.sub(r'/\*.*?\*/', '', content, flags=re.DOTALL)
    # Remove line comments // ...
    result = re.sub(r'//[^\n]*', '', result)
    # Remove single-quoted strings
    result = re.sub(r"'(?:[^'\\]|\\.)*'", "''", result)
    # Remove double-quoted strings
    result = re.sub(r'"(?:[^"\\]|\\.)*"', '""', result)
    # Remove template literals `...`
    result = re.sub(r'`(?:[^`\\]|\\.)*`', '``', result)
    return result


def should_skip(content: str, rel_path: str = "") -> str | None:
    """Check if test should be skipped. Returns reason string or None."""
    # Strip strings and comments to avoid false positives
    code_only = strip_strings_and_comments(content)
    
    # Check skip patterns on code-only content
    for pattern in SKIP_PATTERNS:
        m = re.search(pattern, code_only)
        if m:
            return f"uses {m.group().strip()}"
    
    # Check for typeof on undeclared variable (e.g. typeof x where x is never declared)
    # These tests verify JS's unresolvable reference behavior that Zig can't express.
    reason = _check_typeof_undeclared(code_only)
    if reason:
        return reason
    
    # Check path-based patterns
    reason = _check_path_patterns(rel_path)
    if reason:
        return reason
    
    return None


def _check_typeof_undeclared(code: str) -> str | None:
    """Check if the code uses typeof on an undeclared identifier.
    
    After stripping strings/comments, if typeof is followed by a single identifier
    that never appears as a declaration (var/let/const/function) or on the left
    side of assignment, it's an unresolvable reference.
    """
    # Find all typeof <id> patterns
    typeof_ids = set(re.findall(r'\btypeof\s+([a-zA-Z_$][a-zA-Z_$0-9]*)', code))
    if not typeof_ids:
        return None
    
    # Find all declared/assigned identifiers
    declared = set()
    # var/let/const declarations
    for m in re.finditer(r'\b(?:var|let|const)\s+([a-zA-Z_$][a-zA-Z_$0-9]*)', code):
        declared.add(m.group(1))
    # function declarations
    for m in re.finditer(r'\bfunction\s+([a-zA-Z_$][a-zA-Z_$0-9]*)', code):
        declared.add(m.group(1))
    # Assignment targets (id = ...)
    for m in re.finditer(r'\b([a-zA-Z_$][a-zA-Z_$0-9]*)\s*=(?!=)', code):
        declared.add(m.group(1))
    # for (var id ...)
    for m in re.finditer(r'\bfor\s*\(\s*var\s+([a-zA-Z_$][a-zA-Z_$0-9]*)', code):
        declared.add(m.group(1))
    
    undeclared = typeof_ids - declared
    if undeclared:
        return f"typeof on undeclared variable(s): {', '.join(sorted(undeclared))}"
    return None


def _check_path_patterns(rel_path: str) -> str | None:
    """Check for path-based skip patterns."""
    if 'unresolvable' in rel_path.lower():
        return "unresolvable reference test"
    if 'let-identifier' in rel_path.lower():
        return "let as identifier (ASI edge case)"
    return None


def generate_test_name(rel_path: str) -> str:
    """Generate a valid test function name from a file path.
    
    e.g., language/expressions/addition/S11.6.1_A1.js
         -> test262_language_expressions_addition_S11_6_1_A1
    """
    # Remove .js extension first
    name = rel_path.replace('\\', '/')
    if name.endswith('.js'):
        name = name[:-3]
    # Remove test/ prefix
    if name.startswith('test/'):
        name = name[5:]
    # Replace / with _
    name = name.replace('/', '_')
    # Replace dots with _
    name = name.replace('.', '_')
    # Replace dashes with _
    name = name.replace('-', '_')
    # Prefix with test262_
    name = 'test262_' + name
    # Ensure valid identifier (remove any non-alphanumeric except _)
    name = re.sub(r'[^a-zA-Z0-9_]', '_', name)
    return name


def generate_js_filename(test_name: str) -> str:
    """Generate JS filename from test name."""
    return test_name + '.js'


def scan_directory(test262_dir: str, categories: list[str] | None, max_tests: int | None) -> list[dict]:
    """Scan test262 directory and return list of test entries."""
    base = Path(test262_dir) / 'test'
    
    if categories:
        scan_dirs = [base / c for c in categories]
    else:
        scan_dirs = [base / 'language']
    
    for d in scan_dirs:
        if not d.exists():
            print(f"Error: directory {d} does not exist", file=sys.stderr)
            sys.exit(1)
    
    tests = []
    skipped = []
    seen_names = set()  # deduplicate by test name
    
    for scan_dir in scan_dirs:
        for js_file in sorted(scan_dir.rglob('*.js')):
            rel_path = js_file.relative_to(base)
            rel_str = str(rel_path).replace('\\', '/')
            
            try:
                content = js_file.read_text(encoding='utf-8', errors='replace')
            except Exception as e:
                print(f"  SKIP (read error): {rel_str}: {e}", file=sys.stderr)
                continue
            
            # Check if should skip
            skip_reason = should_skip(content, rel_str)
            if skip_reason:
                skipped.append((rel_str, skip_reason))
                continue
            
            # Transform content
            transformed = transform_content(content)
            
            # Generate test name
            test_name = generate_test_name(rel_str)
            
            # Truncate test name if too long (Zig project name limit is 32 chars,
            # but function names can be longer)
            if len(test_name) > 80:
                import hashlib
                h = hashlib.md5(test_name.encode()).hexdigest()[:8]
                test_name = f"test262_{h}"
            
            # Skip duplicate names (from different directories)
            if test_name in seen_names:
                continue
            seen_names.add(test_name)
            
            tests.append({
                'name': test_name,
                'source': rel_str,
                'content': transformed,
            })
            
            if max_tests and len(tests) >= max_tests:
                break
        
        if max_tests and len(tests) >= max_tests:
            break
    
    return tests, skipped


def write_js_files(project_dir: str, tests: list[dict]):
    """Write transformed JS test files. Clears old generated files first."""
    js_dir = Path(project_dir) / 'js_src'
    js_dir.mkdir(exist_ok=True)
    
    # Clean old generated test files (keep app.js)
    for f in js_dir.glob('test262_*.js'):
        f.unlink()
    
    # Ensure app.js exists (project entry point for Zig name limit)
    app_js = js_dir / 'app.js'
    if not app_js.exists():
        app_js.write_text(
            '// app.js - Main entry point for test262-project.\n'
            '// The project name "app" is derived from this file name.\n'
            'export function app() {\n    return 0;\n}\n'
        )
    
    for test in tests:
        filename = generate_js_filename(test['name'])
        filepath = js_dir / filename
        
        # Build file content: header + harness + test body
        header = f"// {filename}\n// Source: test262/test/{test['source']}\n\n"
        body = test['content'].strip()
        
        # Wrap in export function
        wrapped = f"export function {test['name']}() {{\n{body}\n}}\n"
        
        content = header + HARNESS + '\n' + wrapped
        filepath.write_text(content, encoding='utf-8')


def write_toml(project_dir: str, tests: list[dict]):
    """Write js2rust.toml configuration."""
    toml_path = Path(project_dir) / 'js2rust.toml'
    
    lines = [
        '[project]',
        'js_dir = "js_src"',
        'js_files = [',
        '    "app.js",',
    ]
    
    for test in tests:
        filename = generate_js_filename(test['name'])
        lines.append(f'    "{filename}",')
    
    lines.extend([
        ']',
        '',
        '[build]',
        'force_rebuild = true',
        '',
        '[[host_functions]]',
        'name = "host_assert_fail"',
        'params = ["str"]',
        'returns = "void"',
    ])
    
    toml_path.write_text('\n'.join(lines) + '\n', encoding='utf-8')


def write_main_rs(project_dir: str, tests: list[dict]):
    """Write src/main.rs with auto-generated dispatch table."""
    main_rs_path = Path(project_dir) / 'src' / 'main.rs'
    
    test_names = [t['name'] for t in tests]
    
    lines = [
        '// src/main.rs - test262 conformance test runner (AUTO-GENERATED by scan_test262.py)',
        '//',
        '// Usage:',
        '//   test262-project                    Run all test262 tests',
        '//   test262-project --list             List all available tests',
        '//   test262-project <test_name>        Run a single test (direct call mode)',
        '//   test262-project --all              Same as default',
        '',
        'use js2rust_bridge::js2rust_bridge;',
        'use std::env;',
        'use std::process::Command;',
        '',
        'mod host;',
        '',
        'js2rust_bridge!();',
        '',
        '// Flush C stdio buffers (Zig runtime writes via C FFI, not Rust stdout).',
        'extern "C" {',
        '    fn fflush(stream: *mut std::ffi::c_void) -> i32;',
        '}',
        '',
        'fn flush_stdout() {',
        '    unsafe {',
        '        fflush(std::ptr::null_mut());',
        '    }',
        '}',
        '',
        '/// All test262 test names, matching the export function names in js_src/*.js.',
        'const ALL_TESTS: &[&str] = &[',
    ]
    
    for name in test_names:
        lines.append(f'    "{name}",')
    
    lines.extend([
        '];',
        '',
        'fn main() {',
        '    let args: Vec<String> = env::args().collect();',
        '    let binary = args[0].clone();',
        '',
        '    if args.len() < 2 {',
        '        run_all(&binary);',
        '        return;',
        '    }',
        '',
        '    match args[1].as_str() {',
        '        "--list" => {',
        '            for test in ALL_TESTS {',
        '                println!("{}", test);',
        '            }',
        '        }',
        '        "--all" => {',
        '            run_all(&binary);',
        '        }',
        '        test_name => {',
        '            // Child process mode: call the test function directly.',
        '            js2rust_init();',
        '            if !run_test_direct(test_name) {',
        '                eprintln!("Unknown test: {}", test_name);',
        '                eprintln!("Use --list to see available tests.");',
        '                flush_stdout();',
        '                js2rust_deinit();',
        '                std::process::exit(2);',
        '            }',
        '            flush_stdout();',
        '            js2rust_deinit();',
        '        }',
        '    }',
        '}',
        '',
        '/// Dispatch to a single bridge function. Returns false if test name is unknown.',
        '/// Called directly in child process mode (no further spawning).',
        '#[allow(clippy::let_unit_value)]',
        'fn run_test_direct(test_name: &str) -> bool {',
        '    match test_name {',
    ])
    
    for name in test_names:
        lines.extend([
            f'        "{name}" => {{',
            f'            let _ = {name}();',
            '            true',
            '        }',
        ])
    
    lines.extend([
        '        _ => false,',
        '    }',
        '}',
        '',
        '/// Run all tests via child processes (crash isolation).',
        'fn run_all(binary: &str) {',
        '    let total = ALL_TESTS.len();',
        '    let mut passed = 0usize;',
        '    let mut failed = 0usize;',
        '    let mut errors = 0usize;',
        '    let mut failures: Vec<(String, String)> = Vec::new();',
        '',
        '    for (i, test) in ALL_TESTS.iter().enumerate() {',
        '        flush_stdout();',
        '',
        '        let result = Command::new(binary).arg(test).output();',
        '',
        '        match result {',
        '            Ok(out) => {',
        '                if out.status.success() {',
        '                    passed += 1;',
        '                    eprintln!("[{}/{}] {} ... PASS", i + 1, total, test);',
        '                } else {',
        '                    let stderr = String::from_utf8_lossy(&out.stderr);',
        '                    let first_line = stderr.lines().next().unwrap_or("unknown");',
        '                    failed += 1;',
        '                    eprintln!("[{}/{}] {} ... FAIL ({})", i + 1, total, test, first_line);',
        '                    failures.push((test.to_string(), stderr.to_string()));',
        '                }',
        '            }',
        '            Err(e) => {',
        '                errors += 1;',
        '                eprintln!("[{}/{}] {} ... ERROR (spawn: {})", i + 1, total, test, e);',
        '            }',
        '        }',
        '    }',
        '',
        '    // Summary.',
        '    eprintln!();',
        '    eprintln!("=== test262 Summary ===");',
        '    eprintln!(',
        '        "Total: {}, Passed: {}, Failed: {}, Errors: {}",',
        '        total, passed, failed, errors',
        '    );',
        '',
        '    if !failures.is_empty() {',
        '        eprintln!();',
        '        eprintln!("=== Failures ===");',
        '        for (test, stderr) in &failures {',
        '            eprintln!();',
        '            eprintln!("  {}:", test);',
        '            let lines: Vec<&str> = stderr.trim_end().lines().take(5).collect();',
        '            for line in lines {',
        '                eprintln!("    {}", line);',
        '            }',
        '        }',
        '    }',
        '',
        '    // Exit code 0 regardless - test262-project is a diagnostic tool.',
        '}',
    ])
    
    main_rs_path.write_text('\n'.join(lines) + '\n', encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description='Scan test262 tests and generate js2rust test files')
    parser.add_argument('test262_dir', help='Path to test262 repository')
    parser.add_argument('--category', nargs='+', default=None,
                        help='Category(s) to scan (e.g., language/expressions/addition language/statements/for)')
    parser.add_argument('--max', type=int, default=None,
                        help='Maximum number of tests to generate')
    parser.add_argument('--project-dir', default=None,
                        help='Output project directory (default: auto-detect)')
    
    args = parser.parse_args()
    
    # Auto-detect project directory
    project_dir = args.project_dir or os.path.dirname(os.path.abspath(__file__))
    project_dir = os.path.join(project_dir, '..')
    project_dir = os.path.normpath(project_dir)
    
    cat_str = ' '.join(args.category) if args.category else 'language'
    print(f"Scanning: {args.test262_dir}/test/{cat_str}")
    print(f"Output:   {project_dir}")
    
    tests, skipped = scan_directory(args.test262_dir, args.category, args.max)
    
    print(f"\nResults:")
    print(f"  Generated: {len(tests)} test files")
    print(f"  Skipped:   {len(skipped)} test files")
    
    if skipped:
        print(f"\nSkipped tests (showing first 20):")
        for path, reason in skipped[:20]:
            print(f"  {path}: {reason}")
        if len(skipped) > 20:
            print(f"  ... and {len(skipped) - 20} more")
    
    if not tests:
        print("\nNo tests generated. Check your test262 directory path.")
        return
    
    # Write output files
    write_js_files(project_dir, tests)
    write_toml(project_dir, tests)
    write_main_rs(project_dir, tests)
    
    print(f"\nGenerated files:")
    print(f"  js_src/*.js ({len(tests)} files)")
    print(f"  js2rust.toml")
    print(f"  src/main.rs")


if __name__ == '__main__':
    main()
