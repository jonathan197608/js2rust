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
