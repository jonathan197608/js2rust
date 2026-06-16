// bitwise_ops.js — bitwise operations module
// Exports: bitAnd, bitOr, bitXor, bitNot, bitShift

export function bitAnd(a, b) {
    return a & b;
}

export function bitOr(a, b) {
    return a | b;
}

export function bitXor(a, b) {
    return a ^ b;
}

export function bitNot(a) {
    return ~a;
}

export function bitShift(a, n) {
    return a << n;
}
