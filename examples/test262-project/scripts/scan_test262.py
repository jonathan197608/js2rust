#!/usr/bin/env python3
"""
test262 scanner: transforms test262 test files into js2rust-compatible format.

Usage:
    python scan_test262.py <test262_dir> [--category <path>] [--max <N>]

Scans test262 .js files, applies harness transformations, and generates:
  - js_src/runtime.js     (standalone reference, NOT in js2rust.toml)
  - js_src/test262_*.js   (one file per test, with inline assert helpers)
  - js2rust.toml          (project config)
  - src/main.rs           (test runner with auto-generated dispatch)

Design principles:
  - Minimize JS rewriting: only assert.sameValue -> assert_sameValue
  - Inline assert helpers: each test file self-contained (non-exported → anytype)
  - Per-test isolation: each test exported individually for crash isolation
  - runtime.js: standalone reference for future transpiler improvements
"""

import argparse
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

# ─── runtime.js preamble (shared across all test files) ───
RUNTIME = """\
// --- test262 runtime: shared assert helpers ---
// Non-exported functions → Zig anytype params (Rule 7)
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message !== undefined) { host_assert_fail(message); }
        else { host_assert_fail("assert.sameValue failed"); }
    }
}
function assert_not_same_value(actual, expected, message) {
    if (actual === expected) {
        if (message !== undefined) { host_assert_fail(message); }
        else { host_assert_fail("assert.notSameValue failed"); }
    }
}
"""

# ─── Skip patterns ───
SKIP_PATTERNS = [
    r'\$DONOTEVALUATE',
    r'\$MAX_ITERATIONS',
    r'\beval\s*\(',
    r'\$262',
    r'\basync\s+',
    r'\bawait\s+',
    r'\bimport\s+',
    r'\byield\b',
    r'\bfunction\s*\*',
    r'\bSymbol\b',
    r'\bProxy\b',
    r'\bReflect\b',
    r'\bWeakMap\b',
    r'\bWeakSet\b',
    r'\bSharedArrayBuffer\b',
    r'\bBigInt\b',
    r'\b\d+n\b',
    r'\b0x[0-9a-fA-F]+n\b',
    r'\binstanceof\b',
    r'\bassert\.throws\b',
    r'\bassert\.doesNotThrow\b',
    r'\bassert\.sameValue\s*\(\s*undefined',
    r'\bassert\s*\(',          # bare assert() calls
    r'\.\.\.',                 # rest/spread
    r'\b(const|let|var)\s*\[', # array destructuring
    r'\b(const|let|var)\s*\{', # object destructuring
    r'\{[^}]*\.\.\.',          # object rest destructuring
    r'\btry\s*\{',             # try/catch
    r'\bthis\b',
    r'\b[+-]?0\s*/\s*[+-]?0\b',  # 0/0 → Zig illegal behavior
    r'\bparseInt\b',
    r'\bRegExp\b',
    r'\bDate\b',
    r'\bArray\.isArray\b',
    r'\bnew\s+(?!Test262Error\b)\w+',
    r'\([^)]*\w+\s*=(?![=>])\s*[^=)]',  # assignment-as-expression
    r'\{\s*\}',
    r'\bfunction\s*\([^)]*\)\s*\{',
    r'\b\.valueOf\b',
    r'\b\.toString\s*\(',
    r'\bdelete\s+',
    r'\bvoid\s+',
    r'\bin\s\s',
]

# ─── Helpers ───

def strip_strings_and_comments(content: str) -> str:
    """Remove string literals and comments to avoid false positives in skip matching."""
    result = re.sub(r'/\*.*?\*/', '', content, flags=re.DOTALL)
    result = re.sub(r'//[^\n]*', '', result)
    result = re.sub(r"'(?:[^'\\]|\\.)*'", "''", result)
    result = re.sub(r'"(?:[^"\\]|\\.)*"', '""', result)
    result = re.sub(r'`(?:[^`\\]|\\.)*`', '``', result)
    return result


def should_skip(content: str, rel_path: str = "") -> str | None:
    """Check if test should be skipped. Returns reason string or None."""
    code_only = strip_strings_and_comments(content)

    for pattern in SKIP_PATTERNS:
        m = re.search(pattern, code_only)
        if m:
            return f"uses {m.group().strip()}"

    # typeof on undeclared variables
    reason = _check_typeof_undeclared(code_only)
    if reason:
        return reason

    # Path-based patterns
    if 'unresolvable' in rel_path.lower():
        return "unresolvable reference test"
    if 'let-identifier' in rel_path.lower():
        return "let as identifier (ASI edge case)"

    return None


def _check_typeof_undeclared(code: str) -> str | None:
    """Detect `typeof x` where x is never declared."""
    typeof_ids = set(re.findall(r'\btypeof\s+([a-zA-Z_$][a-zA-Z_$0-9]*)', code))
    if not typeof_ids:
        return None

    declared = set()
    for m in re.finditer(r'\b(?:var|let|const)\s+([a-zA-Z_$][a-zA-Z_$0-9]*)', code):
        declared.add(m.group(1))
    for m in re.finditer(r'\bfunction\s+([a-zA-Z_$][a-zA-Z_$0-9]*)', code):
        declared.add(m.group(1))
    for m in re.finditer(r'\b([a-zA-Z_$][a-zA-Z_$0-9]*)\s*=(?!=)', code):
        declared.add(m.group(1))
    for m in re.finditer(r'\bfor\s*\(\s*var\s+([a-zA-Z_$][a-zA-Z_$0-9]*)', code):
        declared.add(m.group(1))

    undeclared = typeof_ids - declared
    if undeclared:
        return f"typeof on undeclared: {', '.join(sorted(undeclared))}"
    return None


# ─── Transformations ───

def transform_content(content: str) -> str:
    """Apply minimal text transformations to test262 content."""
    # Strip frontmatter
    content = re.sub(r'/\*---.*?---\*/', '', content, flags=re.DOTALL)

    # assert.sameValue -> assert_same_value (only rewriting needed)
    content = re.sub(r'\bassert\.sameValue\s*\(', 'assert_same_value(', content)
    content = re.sub(r'\bassert\.notSameValue\s*\(', 'assert_not_same_value(', content)

    # throw new Test262Error(msg) -> host_assert_fail(msg)
    content = re.sub(r'throw\s+new\s+Test262Error\s*\(', 'host_assert_fail(', content)
    content = re.sub(r'new\s+Test262Error\s*\(', 'host_assert_fail(', content)

    # Fix 2-arg assert calls -> default "" 3rd arg
    for func_name in ('assert_same_value', 'assert_not_same_value'):
        content = _add_default_third_arg(content, func_name + '(', '""')

    return content


def _add_default_third_arg(content: str, prefix: str, default_arg: str) -> str:
    """For 2-arg calls matching prefix, insert default_arg as 3rd arg."""
    result = []
    i = 0
    plen = len(prefix)
    while i < len(content):
        idx = content.find(prefix, i)
        if idx == -1:
            result.append(content[i:])
            break
        result.append(content[i:idx])

        start = idx + plen
        depth = 1
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
            elif ch in ('"', "'"):
                quote = ch
                j += 1
                while j < len(content) and content[j] != quote:
                    if content[j] == '\\':
                        j += 1
                    j += 1
            elif ch == '`':
                j += 1
                while j < len(content) and content[j] != '`':
                    if content[j] == '\\':
                        j += 1
                    j += 1
            j += 1

        if depth == 0 and comma_count == 1:
            result.append(prefix)
            result.append(content[start:j])
            result.append(', ' + default_arg)
            result.append(')')
        else:
            result.append(prefix)
            if depth == 0:
                result.append(content[start:j + 1])
            else:
                result.append(content[start:])
                break

        i = j + 1
    return ''.join(result)


# ─── Name generation ───

def generate_test_name(rel_path: str) -> str:
    """Generate a valid JS function name from test262 relative path."""
    name = rel_path.replace('\\', '/')
    if name.endswith('.js'):
        name = name[:-3]
    if name.startswith('test/'):
        name = name[5:]
    name = name.replace('/', '_').replace('.', '_').replace('-', '_')
    name = 'test262_' + name
    name = re.sub(r'[^a-zA-Z0-9_]', '_', name)
    return name


def category_from_path(rel_path: str) -> str:
    """Extract category slug from relative path.
    e.g., language/expressions/addition/S11.6.1_A1.js -> 'expressions_addition'
    """
    name = rel_path.replace('\\', '/')
    if name.startswith('test/'):
        name = name[5:]
    parts = name.split('/')
    # Take first two path components (skip 'language' if present)
    if parts[0] == 'language':
        parts = parts[1:]
    return '_'.join(parts[:2])


# ─── Scanning ───

def scan_directory(test262_dir: str, categories: list[str] | None,
                   max_tests: int | None) -> tuple[dict[str, list[dict]], list[tuple[str, str]]]:
    """Scan test262 directory, return tests grouped by category."""
    base = Path(test262_dir) / 'test'

    if categories:
        scan_dirs = [base / c for c in categories]
    else:
        scan_dirs = [base / 'language']

    for d in scan_dirs:
        if not d.exists():
            print(f"Error: directory {d} does not exist", file=sys.stderr)
            sys.exit(1)

    grouped = defaultdict(list)  # category -> [test dicts]
    skipped = []
    seen_names = set()
    total = 0

    for scan_dir in scan_dirs:
        for js_file in sorted(scan_dir.rglob('*.js')):
            total += 1
            rel_path = js_file.relative_to(base)
            rel_str = str(rel_path).replace('\\', '/')

            try:
                content = js_file.read_text(encoding='utf-8', errors='replace')
            except Exception as e:
                print(f"  SKIP (read error): {rel_str}: {e}", file=sys.stderr)
                continue

            skip_reason = should_skip(content, rel_str)
            if skip_reason:
                skipped.append((rel_str, skip_reason))
                continue

            transformed = transform_content(content)
            test_name = generate_test_name(rel_str)

            # Truncate long names
            if len(test_name) > 80:
                import hashlib
                h = hashlib.md5(test_name.encode()).hexdigest()[:8]
                test_name = f"test262_{h}"

            if test_name in seen_names:
                continue
            seen_names.add(test_name)

            cat = category_from_path(rel_str)
            grouped[cat].append({
                'name': test_name,
                'source': rel_str,
                'content': transformed,
            })

            if max_tests and sum(len(v) for v in grouped.values()) >= max_tests:
                break

        if max_tests and sum(len(v) for v in grouped.values()) >= max_tests:
            break

    return dict(grouped), skipped


# ─── Output generation ───

# Inline assert helpers (per-file, non-exported → Zig anytype params)
ASSERT_HELPERS = """\
// --- assert helpers (non-exported → Zig anytype) ---
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message !== undefined) { host_assert_fail(message); }
        else { host_assert_fail("assert.sameValue failed"); }
    }
}
function assert_not_same_value(actual, expected, message) {
    if (actual === expected) {
        if (message !== undefined) { host_assert_fail(message); }
        else { host_assert_fail("assert.notSameValue failed"); }
    }
}
"""


def write_runtime(project_dir: str):
    """Write shared runtime.js as standalone reference (NOT in js2rust.toml).
    Each test file includes its own inline assert helpers due to transpiler
    limitation: non-exported functions cannot be shared across JS files.
    """
    js_dir = Path(project_dir) / 'js_src'
    js_dir.mkdir(exist_ok=True)
    (js_dir / 'runtime.js').write_text(RUNTIME, encoding='utf-8')


def write_js_files(project_dir: str, grouped: dict[str, list[dict]]):
    """Write one JS file per test with inline assert helpers."""
    js_dir = Path(project_dir) / 'js_src'
    js_dir.mkdir(exist_ok=True)

    # Clean old generated test files
    for f in js_dir.glob('test262_*.js'):
        f.unlink()
    for f in js_dir.glob('test_*.js'):
        f.unlink()

    # Ensure app.js
    app_js = js_dir / 'app.js'
    if not app_js.exists():
        app_js.write_text(
            '// app.js - Main entry point for test262-project.\n'
            'export function app() { return 0; }\n'
        )

    # Flatten tests while preserving order
    all_tests = []
    for cat in sorted(grouped.keys()):
        all_tests.extend(grouped[cat])

    for test in all_tests:
        filename = test['name'] + '.js'
        filepath = js_dir / filename

        lines = [
            f'// test262 source: test/{test["source"]}',
            f'// AUTO-GENERATED by scan_test262.py',
            '',
            ASSERT_HELPERS.strip(),
            '',
            f'export function {test["name"]}() {{',
        ]

        body = test['content'].strip()
        for line in body.split('\n'):
            lines.append('    ' + line)

        lines.append('}')

        filepath.write_text('\n'.join(lines) + '\n', encoding='utf-8')


def write_toml(project_dir: str, grouped: dict[str, list[dict]]):
    """Write js2rust.toml with individual test files."""
    toml_path = Path(project_dir) / 'js2rust.toml'

    lines = [
        '[project]',
        'js_dir = "js_src"',
        'js_files = [',
        '    "app.js",',
    ]

    # Flatten tests while preserving order
    for cat in sorted(grouped.keys()):
        for test in grouped[cat]:
            lines.append(f'    "{test["name"]}.js",')

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


def write_main_rs(project_dir: str, grouped: dict[str, list[dict]]):
    """Write src/main.rs with auto-generated dispatch table."""
    main_rs_path = Path(project_dir) / 'src' / 'main.rs'

    # Flatten tests while preserving order
    all_tests = []
    for cat in sorted(grouped.keys()):
        all_tests.extend(t['name'] for t in grouped[cat])

    lines = [
        '// src/main.rs - test262 conformance test runner (AUTO-GENERATED)',
        '//',
        f'// Categories: {len(grouped)}, Tests: {len(all_tests)}',
        '',
        'use js2rust_bridge::js2rust_bridge;',
        'use std::env;',
        'use std::process::Command;',
        '',
        'mod host;',
        '',
        'js2rust_bridge!();',
        '',
        'extern "C" { fn fflush(stream: *mut std::ffi::c_void) -> i32; }',
        'fn flush_stdout() { unsafe { fflush(std::ptr::null_mut()); } }',
        '',
        'const ALL_TESTS: &[&str] = &[',
    ]

    for name in all_tests:
        lines.append(f'    "{name}",')

    lines.extend([
        '];',
        '',
        'fn main() {',
        '    let args: Vec<String> = env::args().collect();',
        '    let binary = args[0].clone();',
        '    if args.len() < 2 { run_all(&binary); return; }',
        '    match args[1].as_str() {',
        '        "--list" => { for test in ALL_TESTS { println!("{}", test); } }',
        '        "--all" => run_all(&binary),',
        '        test_name => {',
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
        '#[allow(clippy::let_unit_value)]',
        'fn run_test_direct(test_name: &str) -> bool {',
        '    match test_name {',
    ])

    for name in all_tests:
        lines.append(f'        "{name}" => {{ let _ = {name}(); true }},')

    lines.extend([
        '        _ => false,',
        '    }',
        '}',
        '',
        'fn run_all(binary: &str) {',
        '    let total = ALL_TESTS.len();',
        '    let mut passed = 0usize;',
        '    let mut failed = 0usize;',
        '    let mut errors = 0usize;',
        '    let mut failures: Vec<(String, String)> = Vec::new();',
        '    for (i, test) in ALL_TESTS.iter().enumerate() {',
        '        flush_stdout();',
        '        let result = Command::new(binary).arg(test).output();',
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
        '    eprintln!();',
        '    eprintln!("=== test262 Summary ===");',
        '    eprintln!("Total: {}, Passed: {}, Failed: {}, Errors: {}",',
        '        total, passed, failed, errors);',
        '    if !failures.is_empty() {',
        '        eprintln!();',
        '        eprintln!("=== Failures ===");',
        '        for (test, stderr) in &failures {',
        '            eprintln!();',
        '            eprintln!("  {}:", test);',
        '            for line in stderr.trim_end().lines().take(5) {',
        '                eprintln!("    {}", line);',
        '            }',
        '        }',
        '    }',
        '}',
    ])

    main_rs_path.write_text('\n'.join(lines) + '\n', encoding='utf-8')


# ─── Entry point ───

def main():
    parser = argparse.ArgumentParser(description='Scan test262 tests → merged js2rust test files')
    parser.add_argument('test262_dir', help='Path to test262 repository')
    parser.add_argument('--category', nargs='+', default=None,
                        help='Category(s) to scan (e.g. language/expressions/addition)')
    parser.add_argument('--max', type=int, default=None,
                        help='Maximum number of tests')
    parser.add_argument('--project-dir', default=None,
                        help='Output project directory (default: auto-detect)')

    args = parser.parse_args()

    project_dir = args.project_dir or os.path.normpath(
        os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

    cat_str = ' '.join(args.category) if args.category else 'language'
    print(f"Scanning: {args.test262_dir}/test/{cat_str}")
    print(f"Output:   {project_dir}")

    grouped, skipped = scan_directory(args.test262_dir, args.category, args.max)

    total_tests = sum(len(v) for v in grouped.values())
    print(f"\nResults:")
    print(f"  Categories: {len(grouped)}")
    print(f"  Generated:  {total_tests} tests")
    print(f"  Skipped:    {len(skipped)} files")

    if skipped:
        print(f"\nSkipped (showing first 20):")
        for path, reason in skipped[:20]:
            print(f"  {path}: {reason}")
        if len(skipped) > 20:
            print(f"  ... and {len(skipped) - 20} more")

    if not grouped:
        print("\nNo tests generated.")
        return

    write_runtime(project_dir)
    write_js_files(project_dir, grouped)
    write_toml(project_dir, grouped)
    write_main_rs(project_dir, grouped)

    print(f"\nGenerated files:")
    print(f"  js_src/runtime.js  (standalone reference, NOT in js2rust.toml)")
    for cat in sorted(grouped.keys()):
        count = len(grouped[cat])
        print(f"  js_src/test262_*.js  ({cat}: {count} tests)")
    print(f"  js2rust.toml  ({total_tests} test entries)")
    print(f"  src/main.rs")


if __name__ == '__main__':
    main()
