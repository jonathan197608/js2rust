#!/usr/bin/env python3
"""
test262 scanner: wraps raw test262 test files into js2rust-compatible format.

Usage:
    python scan_test262.py <test_dir> [--category <path>] [--max <N>]

Scans .js test files from a directory (no git clone needed -- just copy
test262 test fragments into any directory and point the scanner at it).

Design principles:
  - NO filtering: all tests are included; unsupported features surface as
    Zig compile errors, which serve as a feature-gap TODO list.
  - NO test modification: raw test code is wrapped verbatim in an export
    function. Only frontmatter (/*---...---*/) is stripped (it is metadata,
    not test code).
  - NO runtime.js: assert is a built-in global injected by the Zig runtime
    preamble (see push_runtime_imports in project.rs). Test files do not
    need any import statement.
  - Per-test isolation: each test is exported individually for crash isolation.
"""

import argparse
import hashlib
import os
import re
import sys
from collections import defaultdict
from pathlib import Path


# ─── Frontmatter stripping ───

def strip_frontmatter(content: str) -> str:
    """Remove test262 frontmatter block (/*--- ... ---*/), keeping all test code."""
    return re.sub(r'/\*---.*?---\*/', '', content, flags=re.DOTALL)


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
    """Extract category slug from relative path."""
    name = rel_path.replace('\\', '/')
    if name.startswith('test/'):
        name = name[5:]
    parts = name.split('/')
    if parts[0] == 'language':
        parts = parts[1:]
    return '_'.join(parts[:2])


# ─── Scanning ───

def scan_directory(test_dir: str, categories: list[str] | None,
                   max_tests: int | None) -> dict[str, list[dict]]:
    """Scan directory for .js test files. No filtering -- include everything.

    The directory can be:
      - A full test262 repo clone (test/language/...)
      - Just a directory of .js files copied from test262
      - Any directory with .js files following test262 conventions
    """
    base = Path(test_dir)

    # If the directory has a test/ subdirectory, use that as base
    if (base / 'test').is_dir():
        base = base / 'test'

    if categories:
        scan_dirs = [base / c for c in categories]
    else:
        # Default: scan everything under base
        scan_dirs = [base]

    for d in scan_dirs:
        if not d.exists():
            print(f"Error: directory {d} does not exist", file=sys.stderr)
            sys.exit(1)

    grouped = defaultdict(list)
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

            # Strip frontmatter only -- no other modifications
            test_code = strip_frontmatter(content)

            test_name = generate_test_name(rel_str)

            # Truncate long names
            if len(test_name) > 80:
                h = hashlib.md5(test_name.encode()).hexdigest()[:8]
                test_name = f"test262_{h}"

            # Handle name collisions
            if test_name in seen_names:
                continue
            seen_names.add(test_name)

            cat = category_from_path(rel_str)
            grouped[cat].append({
                'name': test_name,
                'source': rel_str,
                'content': test_code,
            })

            if max_tests and sum(len(v) for v in grouped.values()) >= max_tests:
                break

        if max_tests and sum(len(v) for v in grouped.values()) >= max_tests:
            break

    return dict(grouped)


# ─── Output generation ───

def write_js_files(project_dir: str, grouped: dict[str, list[dict]]):
    """Write one JS file per test. Raw test code is wrapped verbatim -- no modifications.

    No import statement needed: `assert` is a built-in global provided by the
    Zig runtime preamble (push_runtime_imports in project.rs).
    No runtime.js: assert functions are injected directly into each .zig file.
    """
    js_dir = Path(project_dir) / 'js_src'
    js_dir.mkdir(exist_ok=True)

    # Clean old generated test files
    for f in js_dir.glob('test262_*.js'):
        f.unlink()
    for f in js_dir.glob('test_*.js'):
        f.unlink()

    # Remove old runtime.js if it exists
    old_runtime = js_dir / 'runtime.js'
    if old_runtime.exists():
        old_runtime.unlink()

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
            f'// test262 source: {test["source"]}',
            f'// AUTO-GENERATED by scan_test262.py',
            '',
            f'export function {test["name"]}() {{',
        ]

        # Raw test code -- no modifications, just indented
        body = test['content'].strip()
        for line in body.split('\n'):
            lines.append('    ' + line if line else '')

        lines.append('}')

        filepath.write_text('\n'.join(lines) + '\n', encoding='utf-8')


def write_toml(project_dir: str, grouped: dict[str, list[dict]]):
    """Write js2rust.toml with individual test files.

    No runtime.js entry -- assert is a built-in global in the Zig runtime.
    """
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
        'run_zig_build = false',
        '',
        '[[host_functions]]',
        'name = "host_assert_fail"',
        'params = ["str"]',
        'returns = "void"',
    ])

    toml_path.write_text('\n'.join(lines) + '\n', encoding='utf-8')


def write_main_rs(project_dir: str, grouped: dict[str, list[dict]]):
    """Write src/main.rs with auto-generated dispatch table.

    Includes SKIP infrastructure: tests that fail transpilation are listed in
    SKIPPED_TESTS and handled at runtime (exit code 3 = SKIP). This set is
    empty by default — populate it after the first build reveals which tests
    the transpiler cannot handle.
    """
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
        '/// Tests that failed transpilation (unsupported JS features).',
        '/// Their C ABI functions are not generated, so we skip them at runtime.',
        '/// Populate this list after the first build by checking which tests',
        '/// the transpiler skipped (look for "skip" in build output).',
        'const SKIPPED_TESTS: &[(&str, &str)] = &[',
        '    // ("test_name", "reason"),',
        '];',
        '',
        'fn is_skipped(name: &str) -> Option<&\'static str> {',
        '    SKIPPED_TESTS.iter()',
        '        .find(|(n, _)| *n == name)',
        '        .map(|(_, reason)| *reason)',
        '}',
        '',
        'const ALL_TESTS: &[&str] = &[',
    ]

    for name in all_tests:
        lines.append(f'    "{name}",')

    lines.extend([
        '];',
        '',
        'enum TestResult {',
        '    Ran,',
        '    Skipped(&\'static str),',
        '    Unknown,',
        '}',
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
        '            match run_test_direct(test_name) {',
        '                TestResult::Ran => {}',
        '                TestResult::Skipped(reason) => {',
        '                    eprintln!("SKIP: {}", reason);',
        '                    flush_stdout();',
        '                    js2rust_deinit();',
        '                    std::process::exit(3);',
        '                }',
        '                TestResult::Unknown => {',
        '                    eprintln!("Unknown test: {}", test_name);',
        '                    eprintln!("Use --list to see available tests.");',
        '                    flush_stdout();',
        '                    js2rust_deinit();',
        '                    std::process::exit(2);',
        '                }',
        '            }',
        '            flush_stdout();',
        '            js2rust_deinit();',
        '        }',
        '    }',
        '}',
        '',
        '#[allow(clippy::let_unit_value)]',
        'fn run_test_direct(test_name: &str) -> TestResult {',
        '    if let Some(reason) = is_skipped(test_name) {',
        '        return TestResult::Skipped(reason);',
        '    }',
        '    match test_name {',
    ])

    for name in all_tests:
        lines.append(f'        "{name}" => {{ let _ = {name}(); TestResult::Ran }},')

    lines.extend([
        '        _ => TestResult::Unknown,',
        '    }',
        '}',
        '',
        'fn run_all(binary: &str) {',
        '    let total = ALL_TESTS.len();',
        '    let mut passed = 0usize;',
        '    let mut failed = 0usize;',
        '    let mut errors = 0usize;',
        '    let mut skipped = 0usize;',
        '    let mut failures: Vec<(String, String)> = Vec::new();',
        '    for (i, test) in ALL_TESTS.iter().enumerate() {',
        '        flush_stdout();',
        '        let result = Command::new(binary).arg(test).output();',
        '        match result {',
        '            Ok(out) => {',
        '                let code = out.status.code();',
        '                if code == Some(3) {',
        '                    skipped += 1;',
        '                    eprintln!("[{}/{}] {} ... SKIP", i + 1, total, test);',
        '                } else if out.status.success() {',
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
        '    eprintln!("Total: {}, Passed: {}, Failed: {}, Skipped: {}, Errors: {}",',
        '        total, passed, failed, skipped, errors);',
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
    parser = argparse.ArgumentParser(
        description='Scan test262 test files -> js2rust test files (no filtering, no modification)')
    parser.add_argument('test_dir', help='Path to directory containing test .js files')
    parser.add_argument('--category', nargs='+', default=None,
                        help='Category subdirectory(s) to scan (e.g. language/expressions/addition)')
    parser.add_argument('--max', type=int, default=None,
                        help='Maximum number of tests to generate')
    parser.add_argument('--project-dir', default=None,
                        help='Output project directory (default: auto-detect)')

    args = parser.parse_args()

    project_dir = args.project_dir or os.path.normpath(
        os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

    cat_str = ' '.join(args.category) if args.category else '(all)'
    print(f"Scanning: {args.test_dir} / {cat_str}")
    print(f"Output:   {project_dir}")

    grouped = scan_directory(args.test_dir, args.category, args.max)

    total_tests = sum(len(v) for v in grouped.values())
    print(f"\nResults:")
    print(f"  Categories: {len(grouped)}")
    print(f"  Generated:  {total_tests} tests")
    print(f"  (no filtering -- unsupported features will surface as Zig compile errors)")

    if not grouped:
        print("\nNo tests generated.")
        return

    write_js_files(project_dir, grouped)
    write_toml(project_dir, grouped)
    write_main_rs(project_dir, grouped)

    print(f"\nGenerated files:")
    for cat in sorted(grouped.keys()):
        count = len(grouped[cat])
        print(f"  js_src/test262_*.js  ({cat}: {count} tests)")
    print(f"  js2rust.toml  ({total_tests} test entries + host_assert_fail)")
    print(f"  src/main.rs")
    print(f"\nNote: assert is a built-in global in the Zig runtime preamble.")
    print(f"      No runtime.js or import statements needed.")


if __name__ == '__main__':
    main()
