// src/host.rs — Host function implementations for test262-project.
//
// Only one host function: host_assert_fail(msg: str) -> void.
// All assertion logic is in JS (harness.js), which calls this on failure.

use js2rust_bridge::host_fn;
use js2rust_bridge::sdk::HostStr;

/// Called when a test262 assertion fails.
/// Prints the failure message to stderr and exits with non-zero status.
#[host_fn]
fn host_assert_fail(msg: HostStr) {
    eprintln!("ASSERTION FAILED: {}", &*msg);
    std::process::exit(1);
}
