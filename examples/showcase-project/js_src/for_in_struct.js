// for_in_struct.js — Demonstrates for-in loop with static struct (compile-time unrolled)
// Feature: P2 #6 — for-in static struct integration
// The transpiler detects that `obj` is a static struct (known fields at compile time)
// and unrolls the for-in loop into one block per field.

/**
 * @param {Object} user - Static struct with fields: name, age, email
 * @returns {string}
 */
export function listUserFields(user) {
    var result = "";
    for (var key in user) {
        result = result + key + ":" + user[key] + ";";
    }
    return result;
}

/**
 * @returns {string}
 */
export function demoForInStruct() {
    const user = { name: "Alice", age: 30, email: "alice@example.com" };
    return listUserFields(user);
}
