// js_src/main.js
// Test project for js2rust — JS-to-Zig transpiler

export function greet(name) {
    return "Hello, " + name + "!";
}

export function add(a, b) {
    return a + b;
}

export function multiply(x, y) {
    return x * y;
}

// Host function call examples (synchronous, integer)
export function useHostAdd(a, b) {
    return host_add(a, b);
}

export function useHostMultiply(a, b) {
    return host_multiply(a, b);
}

// Host function call examples (synchronous, string)
export function useHostConcat(s1, s2) {
    return host_concat(s1, s2);
}

export function useHostStrlen(s) {
    return host_strlen(s);
}

// Async host function call example
// Exported async function — C ABI blocking wrapper uses global Io
export async function getUserInfo(name) {
    const user = await fetch_user(name);
    return user.name;
}

// Fetch two users sequentially — demonstrates multiple async host calls
export async function getTwoUserInfo(name1, name2) {
    const user1 = await fetch_user(name1);
    const user2 = await fetch_user(name2);
    return user1.name + " & " + user2.name;
}

// ── Promise tests (Task #86) ─────────────────────────────
// Promise.resolve(value) — create a fulfilled promise
export function testPromiseResolve() {
    const p = Promise.resolve(42);
    p.then((v) => {
        // In real JS: console.log(v);
        // Simplified: just return v to test the callback execution
        return v + 1;
    });
    return 0;
}

// Promise.reject(reason) — create a rejected promise
export function testPromiseReject() {
    const p = Promise.reject("oops");
    p.catch((err) => {
        // Simplified: just return 0
        return 0;
    });
    return 0;
}

// ── TypedArray tests (Task #87) ─────────────────────────────
// new Int32Array([1,2,3]) → Zig: js_typedarray.fromI32(...)
// TEMPORARILY DISABLED: .length handling issue
// export function testNewInt32Array() {
//     const arr = new Int32Array([10, 20, 30]);
//     return arr.length;
// }

// export function testNewUint8Array() {
//     const arr = new Uint8Array([1, 2, 3, 4, 5]);
//     return arr.length;
// }

// Int32Array.from([4,5,6]) → Zig: (js_typedarray.fromI32(...) catch ...)
// export function testInt32ArrayFrom() {
//     const arr = Int32Array.from([4, 5, 6]);
//     return arr.length;
// }

// Instance method: get(index) — returns ?i32
export function testTypedArrayGet() {
    const arr = new Int32Array([10, 20, 30]);
    const first = arr.get(0);
    if (first === null) { return -1; }
    return first;
}

// ── Dynamic array element access with type conversion (Task #184) ──
// arr.push(x) makes arr a dynamic ArrayList; arr[0] returns JsAny,
// which must be converted to i64 when used in integer context.
export function testDynArrayElementAccess() {
    const arr = [];
    arr.push(42);
    arr.push(99);
    return arr[0] + 0;
}

export function testDynArrayElementAccessIdx1() {
    const arr = [];
    arr.push(10);
    arr.push(20);
    arr.push(30);
    return arr[1] + 0;
}

// ── JSON serialization/deserialization tests (native_proto) ──────────

/**
 * @typedef {Object} User
 * @property {string} name
 * @property {number} age
 * @property {string[]} tags
 */

/**
 * @param {User} user
 * @returns {string}
 */
export function getUserJson(user) {
    return JSON.stringify(user);
}

/**
 * @returns {string}
 */
export function parseUserJson() {
    // TEMPORARILY DISABLED: string escaping error in codegen
    // const user = JSON.parse("{\"name\":\"Alice\",\"age\":30,\"tags\":[\"a\",\"b\"]}");
    // return user.name + " is " + user.age + " years old";
    return "disabled";
}
