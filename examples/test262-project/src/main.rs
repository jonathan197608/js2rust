// src/main.rs - test262 conformance test runner (AUTO-GENERATED)
//
// Categories: 1, Tests: 48

use js2rust_bridge::js2rust_bridge;
use std::env;
use std::process::Command;

mod host;

js2rust_bridge!();

extern "C" {
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}
fn flush_stdout() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

/// Tests that failed transpilation (unsupported JS features).
/// Their C ABI functions are not generated, so we skip them at runtime.
const SKIPPED_TESTS: &[(&str, &str)] = &[
    (
        "test262_language_expressions_addition_bigint_errors",
        "computed property key",
    ),
    (
        "test262_language_expressions_addition_bigint_toprimitive",
        "nested closure capture + computed property key",
    ),
    (
        "test262_language_expressions_addition_bigint_wrapped_values",
        "computed property key",
    ),
    (
        "test262_language_expressions_addition_coerce_symbol_to_prim_err",
        "nested closure capture + uninit variable",
    ),
    (
        "test262_language_expressions_addition_coerce_symbol_to_prim_invocation",
        "this outside class + nested closure",
    ),
    (
        "test262_language_expressions_addition_coerce_symbol_to_prim_return_obj",
        "unsupported transpilation",
    ),
    (
        "test262_language_expressions_addition_coerce_symbol_to_prim_return_prim",
        "unsupported transpilation",
    ),
    (
        "test262_language_expressions_addition_order_of_evaluation",
        "nested closure capture",
    ),
];

fn is_skipped(name: &str) -> Option<&'static str> {
    SKIPPED_TESTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, reason)| *reason)
}

const ALL_TESTS: &[&str] = &[
    "test262_language_expressions_addition_bigint_and_number",
    "test262_language_expressions_addition_bigint_arithmetic",
    "test262_language_expressions_addition_bigint_errors",
    "test262_language_expressions_addition_bigint_toprimitive",
    "test262_language_expressions_addition_bigint_wrapped_values",
    "test262_language_expressions_addition_coerce_bigint_to_string",
    "test262_language_expressions_addition_coerce_symbol_to_prim_err",
    "test262_language_expressions_addition_coerce_symbol_to_prim_invocation",
    "test262_language_expressions_addition_coerce_symbol_to_prim_return_obj",
    "test262_language_expressions_addition_coerce_symbol_to_prim_return_prim",
    "test262_language_expressions_addition_get_symbol_to_prim_err",
    "test262_language_expressions_addition_order_of_evaluation",
    "test262_language_expressions_addition_S11_6_1_A1",
    "test262_language_expressions_addition_S11_6_1_A2_1_T1",
    "test262_language_expressions_addition_S11_6_1_A2_1_T2",
    "test262_language_expressions_addition_S11_6_1_A2_1_T3",
    "test262_language_expressions_addition_S11_6_1_A2_2_T1",
    "test262_language_expressions_addition_S11_6_1_A2_2_T2",
    "test262_language_expressions_addition_S11_6_1_A2_2_T3",
    "test262_language_expressions_addition_S11_6_1_A2_3_T1",
    "test262_language_expressions_addition_S11_6_1_A2_4_T1",
    "test262_language_expressions_addition_S11_6_1_A2_4_T2",
    "test262_language_expressions_addition_S11_6_1_A2_4_T3",
    "test262_language_expressions_addition_S11_6_1_A2_4_T4",
    "test262_language_expressions_addition_S11_6_1_A3_1_T1_1",
    "test262_language_expressions_addition_S11_6_1_A3_1_T1_2",
    "test262_language_expressions_addition_S11_6_1_A3_1_T1_3",
    "test262_language_expressions_addition_S11_6_1_A3_1_T2_1",
    "test262_language_expressions_addition_S11_6_1_A3_1_T2_2",
    "test262_language_expressions_addition_S11_6_1_A3_1_T2_3",
    "test262_language_expressions_addition_S11_6_1_A3_1_T2_4",
    "test262_language_expressions_addition_S11_6_1_A3_1_T2_5",
    "test262_language_expressions_addition_S11_6_1_A3_2_T1_1",
    "test262_language_expressions_addition_S11_6_1_A3_2_T1_2",
    "test262_language_expressions_addition_S11_6_1_A3_2_T2_1",
    "test262_language_expressions_addition_S11_6_1_A3_2_T2_2",
    "test262_language_expressions_addition_S11_6_1_A3_2_T2_3",
    "test262_language_expressions_addition_S11_6_1_A3_2_T2_4",
    "test262_language_expressions_addition_S11_6_1_A4_T1",
    "test262_language_expressions_addition_S11_6_1_A4_T2",
    "test262_language_expressions_addition_S11_6_1_A4_T3",
    "test262_language_expressions_addition_S11_6_1_A4_T4",
    "test262_language_expressions_addition_S11_6_1_A4_T5",
    "test262_language_expressions_addition_S11_6_1_A4_T6",
    "test262_language_expressions_addition_S11_6_1_A4_T7",
    "test262_language_expressions_addition_S11_6_1_A4_T8",
    "test262_language_expressions_addition_S11_6_1_A4_T9",
    "test262_language_expressions_addition_symbol_to_string",
];

enum TestResult {
    Ran,                   // test function called successfully
    Skipped(&'static str), // transpilation failed, skip
    Unknown,               // test name not found
}

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
        "--all" => run_all(&binary),
        test_name => {
            js2rust_init();
            match run_test_direct(test_name) {
                TestResult::Ran => {}
                TestResult::Skipped(reason) => {
                    eprintln!("SKIP: {}", reason);
                    flush_stdout();
                    js2rust_deinit();
                    std::process::exit(3);
                }
                TestResult::Unknown => {
                    eprintln!("Unknown test: {}", test_name);
                    eprintln!("Use --list to see available tests.");
                    flush_stdout();
                    js2rust_deinit();
                    std::process::exit(2);
                }
            }
            flush_stdout();
            js2rust_deinit();
        }
    }
}

#[allow(clippy::let_unit_value)]
fn run_test_direct(test_name: &str) -> TestResult {
    if let Some(reason) = is_skipped(test_name) {
        return TestResult::Skipped(reason);
    }
    match test_name {
        "test262_language_expressions_addition_bigint_and_number" => {
            let _ = test262_language_expressions_addition_bigint_and_number();
            TestResult::Ran
        }
        "test262_language_expressions_addition_bigint_arithmetic" => {
            let _ = test262_language_expressions_addition_bigint_arithmetic();
            TestResult::Ran
        }
        // bigint_errors, bigint_toprimitive, bigint_wrapped_values: SKIPPED (transpilation failed)
        "test262_language_expressions_addition_coerce_bigint_to_string" => {
            let _ = test262_language_expressions_addition_coerce_bigint_to_string();
            TestResult::Ran
        }
        // coerce_symbol_to_prim_err, _invocation, _return_obj, _return_prim: SKIPPED (transpilation failed)
        "test262_language_expressions_addition_get_symbol_to_prim_err" => {
            let _ = test262_language_expressions_addition_get_symbol_to_prim_err();
            TestResult::Ran
        }
        // order_of_evaluation: SKIPPED (transpilation failed)
        "test262_language_expressions_addition_S11_6_1_A1" => {
            let _ = test262_language_expressions_addition_S11_6_1_A1();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_1_T1" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_1_T1();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_1_T2" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_1_T2();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_1_T3" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_1_T3();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_2_T1" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_2_T1();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_2_T2" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_2_T2();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_2_T3" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_2_T3();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_3_T1" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_3_T1();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_4_T1" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_4_T1();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_4_T2" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_4_T2();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_4_T3" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_4_T3();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A2_4_T4" => {
            let _ = test262_language_expressions_addition_S11_6_1_A2_4_T4();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_1_T1_1" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_1_T1_1();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_1_T1_2" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_1_T1_2();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_1_T1_3" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_1_T1_3();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_1_T2_1" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_1_T2_1();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_1_T2_2" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_1_T2_2();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_1_T2_3" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_1_T2_3();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_1_T2_4" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_1_T2_4();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_1_T2_5" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_1_T2_5();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_2_T1_1" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_2_T1_1();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_2_T1_2" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_2_T1_2();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_2_T2_1" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_2_T2_1();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_2_T2_2" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_2_T2_2();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_2_T2_3" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_2_T2_3();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A3_2_T2_4" => {
            let _ = test262_language_expressions_addition_S11_6_1_A3_2_T2_4();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A4_T1" => {
            let _ = test262_language_expressions_addition_S11_6_1_A4_T1();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A4_T2" => {
            let _ = test262_language_expressions_addition_S11_6_1_A4_T2();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A4_T3" => {
            let _ = test262_language_expressions_addition_S11_6_1_A4_T3();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A4_T4" => {
            let _ = test262_language_expressions_addition_S11_6_1_A4_T4();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A4_T5" => {
            let _ = test262_language_expressions_addition_S11_6_1_A4_T5();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A4_T6" => {
            let _ = test262_language_expressions_addition_S11_6_1_A4_T6();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A4_T7" => {
            let _ = test262_language_expressions_addition_S11_6_1_A4_T7();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A4_T8" => {
            let _ = test262_language_expressions_addition_S11_6_1_A4_T8();
            TestResult::Ran
        }
        "test262_language_expressions_addition_S11_6_1_A4_T9" => {
            let _ = test262_language_expressions_addition_S11_6_1_A4_T9();
            TestResult::Ran
        }
        "test262_language_expressions_addition_symbol_to_string" => {
            let _ = test262_language_expressions_addition_symbol_to_string();
            TestResult::Ran
        }
        _ => TestResult::Unknown,
    }
}

fn run_all(binary: &str) {
    let total = ALL_TESTS.len();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut errors = 0usize;
    let mut skipped = 0usize;
    let mut failures: Vec<(String, String)> = Vec::new();
    for (i, test) in ALL_TESTS.iter().enumerate() {
        flush_stdout();
        let result = Command::new(binary).arg(test).output();
        match result {
            Ok(out) => {
                let code = out.status.code();
                if code == Some(3) {
                    skipped += 1;
                    eprintln!("[{}/{}] {} ... SKIP", i + 1, total, test);
                } else if out.status.success() {
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
    eprintln!();
    eprintln!("=== test262 Summary ===");
    eprintln!(
        "Total: {}, Passed: {}, Failed: {}, Skipped: {}, Errors: {}",
        total, passed, failed, skipped, errors
    );
    if !failures.is_empty() {
        eprintln!();
        eprintln!("=== Failures ===");
        for (test, stderr) in &failures {
            eprintln!();
            eprintln!("  {}:", test);
            for line in stderr.trim_end().lines().take(5) {
                eprintln!("    {}", line);
            }
        }
    }
}
