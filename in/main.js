// main.js — entry point that imports all modules
// Contains test_ variables for Zig test generation

import { add, multiply, factorial, clamp, makeAdder } from './math.js';
import { voidFunc } from './utils.js';
import { greet } from './strings.js';
import { fetchData, processItems } from './async_ops.js';
import { bitAnd, bitOr, bitXor, bitNot, bitShift } from './bitwise_ops.js';
import { createMultiplier, createOperations } from './closures.js';

// test_ variables — generate Zig test assertions
// These are evaluated by Boa engine to extract expected values
// Then stripped from generated Zig code (test_ prefix)
const test_add = add(3, 5);
const test_multiply = multiply(6, 7);
const test_greet = greet("world");
const test_factorial = factorial(5);
const test_voidFunc = () => voidFunc();
const test_clamp = clamp(15, 0, 10);
const test_makeAdder_result = makeAdder(10)(5);
const test_bitAnd = bitAnd(0xFF, 0x0F);
const test_bitOr = bitOr(0xFF, 0x0F);
const test_bitXor = bitXor(0xFF, 0x0F);
const test_bitNot = bitNot(0x0F);
const test_bitShift = bitShift(1, 4);
const test_createMultiplier = createMultiplier(7);
const test_createOperations = createOperations(10);
