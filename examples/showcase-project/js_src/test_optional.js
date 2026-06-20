// Test optional chaining
export function testOptionalChain() {
    const obj = { prop: 42 };
    const val = obj?.prop;
    if (val === 42) {
        return 1;
    }
    return 0;
}

export function testOptionalChainNull() {
    const obj = null;
    const val = obj?.prop;
    if (val === undefined) {
        return 1;
    }
    return 0;
}
