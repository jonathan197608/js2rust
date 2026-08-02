// src/main.rs — test262 conformance test runner
//
// Usage:
//   test262-project                    Run all test262 tests
//   test262-project --list             List all available tests
//   test262-project <test_name>        Run a single test (direct call mode)
//   test262-project --all              Same as default
//
// Each test function is transpiled from JS to Zig at build time via
// js2rust_bridge!(). At runtime, each function is called in a child
// process for crash isolation. Exit code 0 = pass, non-zero = fail.

use js2rust_bridge::js2rust_bridge;
use std::env;
use std::process::Command;

mod host;

js2rust_bridge!();

// Flush C stdio buffers (Zig runtime writes via C FFI, not Rust stdout).
extern "C" {
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}

fn flush_stdout() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

/// All test262 test names, matching the export function names in js_src/*.js.
const ALL_TESTS: &[&str] = &[
    "test262_language_expressions_addition",
    "test262_language_expressions_multiplication",
    "test262_language_expressions_string_concat",
    "test262_language_expressions_strict_equality",
    "test262_language_expressions_logical_and",
    "test262_language_statements_if_else",
    "test262_language_statements_for_loop",
    "test262_language_statements_while",
    "test262_language_literals_numeric",
];

fn main() {
    let args: Vec<String> = env::args().collect();
    let binary = args[0].clone();

    if args.len() < 2 {
        run_all(&binary);
        return;
    }

    match args[1].as_str() {
        "--list" => {
            for test in ALL_TESTS {
                println!("{}", test);
            }
        }
        "--all" => {
            run_all(&binary);
        }
        test_name => {
            // Child process mode: call the test function directly.
            js2rust_init();
            if !run_test_direct(test_name) {
                eprintln!("Unknown test: {}", test_name);
                eprintln!("Use --list to see available tests.");
                flush_stdout();
                js2rust_deinit();
                std::process::exit(2);
            }
            flush_stdout();
            js2rust_deinit();
        }
    }
}

/// Dispatch to a single bridge function. Returns false if test name is unknown.
/// Called directly in child process mode (no further spawning).
#[allow(clippy::let_unit_value)]
fn run_test_direct(test_name: &str) -> bool {
    match test_name {
        "test262_language_expressions_addition" => {
            let _ = test262_language_expressions_addition();
            true
        }
        "test262_language_expressions_multiplication" => {
            let _ = test262_language_expressions_multiplication();
            true
        }
        "test262_language_expressions_string_concat" => {
            let _ = test262_language_expressions_string_concat();
            true
        }
        "test262_language_expressions_strict_equality" => {
            let _ = test262_language_expressions_strict_equality();
            true
        }
        "test262_language_expressions_logical_and" => {
            let _ = test262_language_expressions_logical_and();
            true
        }
        "test262_language_statements_if_else" => {
            let _ = test262_language_statements_if_else();
            true
        }
        "test262_language_statements_for_loop" => {
            let _ = test262_language_statements_for_loop();
            true
        }
        "test262_language_statements_while" => {
            let _ = test262_language_statements_while();
            true
        }
        "test262_language_literals_numeric" => {
            let _ = test262_language_literals_numeric();
            true
        }
        _ => false,
    }
}

/// Run all tests via child processes (crash isolation).
fn run_all(binary: &str) {
    let total = ALL_TESTS.len();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut errors = 0usize;
    let mut failures: Vec<(String, String)> = Vec::new();

    for (i, test) in ALL_TESTS.iter().enumerate() {
        flush_stdout();

        let result = Command::new(binary).arg(test).output();

        match result {
            Ok(out) => {
                if out.status.success() {
                    passed += 1;
                    eprintln!("[{}/{}] {} ... PASS", i + 1, total, test);
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let first_line = stderr.lines().next().unwrap_or("unknown");
                    failed += 1;
                    eprintln!("[{}/{}] {} ... FAIL ({})", i + 1, total, test, first_line);
                    failures.push((test.to_string(), stderr.to_string()));
                }
            }
            Err(e) => {
                errors += 1;
                eprintln!("[{}/{}] {} ... ERROR (spawn: {})", i + 1, total, test, e);
            }
        }
    }

    // Summary.
    eprintln!();
    eprintln!("=== test262 Summary ===");
    eprintln!(
        "Total: {}, Passed: {}, Failed: {}, Errors: {}",
        total, passed, failed, errors
    );

    if !failures.is_empty() {
        eprintln!();
        eprintln!("=== Failures ===");
        for (test, stderr) in &failures {
            eprintln!();
            eprintln!("  {}:", test);
            let lines: Vec<&str> = stderr.trim_end().lines().take(5).collect();
            for line in lines {
                eprintln!("    {}", line);
            }
        }
    }

    // Exit code 0 regardless — test262-project is a diagnostic tool.
}
