// js_src/main.js — Test project for js2rust
// Sync functions only (async temporarily disabled for #[host_fn] macro merge)

/**
 * @param {string} name
 * @returns {string}
 */
export function greet(name) {
    return "Hello, " + name + "!";
}

/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function add(a, b) {
    return a + b;
}

// ── Sync host function examples ──

/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function useHostAdd(a, b) {
    return host_add(a, b);
}

/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function useHostMultiply(a, b) {
    return host_multiply(a, b);
}

/**
 * @param {string} s1
 * @param {string} s2
 * @returns {string}
 */
export function useHostConcat(s1, s2) {
    return host_concat(s1, s2);
}

/**
 * @param {string} s
 * @returns {number}
 */
export function useHostStrlen(s) {
    return host_strlen(s);
}

// ── Async host function example ──

/**
 * @param {string} name
 */
export async function getUserInfo(name) {
    return await fetch_user(name);
}
