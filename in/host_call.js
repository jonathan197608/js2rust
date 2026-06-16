// host_call.js — JS tests for calling Rust host functions

// Test basic host function call: addition in Rust
function testHostAdd() {
    return hostAdd(10, 20);
}

// Test host function call with arithmetic in JS
function testHostMath() {
    const sum = hostAdd(5, 7);
    return hostMultiply(sum, 3);
}

// Test host function call as return value directly
function testHostMultiply() {
    return hostMultiply(6, 7);
}

// Async host function: await fetchUser("Alice") → UserInfo { id, name }
async function testHostAsync() {
    const user = await fetchUser("Alice");
    return user.id;
}

export { testHostAdd, testHostMath, testHostMultiply, testHostAsync };

// NOTE: test_* variables are NOT used here because host functions
// (hostAdd, hostMultiply, fetchUser) are Rust FFI functions not available
// in boa_engine. Host functions are tested via:
//   - sys unit tests (sys/src/lib.rs)
//   - test/ crate FFI cross-verification
