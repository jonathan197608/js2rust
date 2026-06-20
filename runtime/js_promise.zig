//! js_promise — Minimal Promise support for js2rust
//!
//! Minimum viable implementation (synchronous):
//!
//! ```zig
//! const Promise = js_promise.Promise;
//!
//! // Create a resolved promise
//! var p = Promise.resolve(jsany.fromI64(42));
//!
//! // Attach a callback (executes immediately since already fulfilled)
//! p.then(struct {
//!     fn call(_: @This(), v: JsAny) void {
//!         js_console.log(v);
//!     }
//! }.{});
//! ```

const std = @import("std");
const JsAny = @import("jsany.zig").JsAny;

/// Promise state.
const State = enum {
    pending,
    fulfilled,
    rejected,
};

/// Minimum viable Promise for js2rust.
/// This implementation executes callbacks synchronously (no async microtask queue).
///
/// In JS→Zig translated code, callbacks are Zig structs with a `call(self, JsAny) void` method.
/// Example: `(v) => console.log(v)` becomes:
///   struct { fn call(_: @This(), v: JsAny) void { js_console.log(v); } }.{}
pub const Promise = struct {
    state: State,
    value: JsAny,

    /// Create a resolved promise.
    /// Generated Zig: `js_runtime.Promise.resolve(jsany.fromI64(42))`
    /// Accepts any type via comptime (auto-wraps primitives to JsAny).
    pub fn resolve(value: anytype) Promise {
        return Promise{
            .state = .fulfilled,
            .value = JsAny.from(value),
        };
    }

    /// Create a rejected promise.
    /// Generated Zig: `js_runtime.Promise.reject(jsany.fromStr("error"))`
    pub fn reject(reason: anytype) Promise {
        return Promise{
            .state = .rejected,
            .value = JsAny.from(reason),
        };
    }

    /// Attach a fulfillment callback.
    /// `on_fulfilled` must be a struct with a `call(self, JsAny) void` method.
    /// If the promise is already fulfilled, the callback is called immediately.
    /// If still pending, the callback is stored for later (not yet implemented).
    pub fn then(self: Promise, on_fulfilled: anytype) void {
        if (self.state == .fulfilled) {
            _ = on_fulfilled.call(self.value);
        }
        // TODO: store callback if pending (requires heap allocation)
    }

    /// Attach a rejection callback.
    /// `on_rejected` must be a struct with a `call(self, JsAny) void` method.
    pub fn @"catch"(self: Promise, on_rejected: anytype) void {
        if (self.state == .rejected) {
            _ = on_rejected.call(self.value);
        }
    }
};
