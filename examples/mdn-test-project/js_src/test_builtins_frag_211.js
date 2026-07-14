// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 211
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_211.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_211() {
    // BigInt constructor with number, decimal, hex, octal, binary
    const alsoHuge = BigInt(9007199254740991);
    console.log(alsoHuge);

    const hugeString = BigInt("9007199254740991");
    console.log(hugeString);

    const hugeHex = BigInt("0x1fffffffffffff");
    console.log(hugeHex);

    const hugeOctal = BigInt("0o377777777777777777");
    console.log(hugeOctal);

    const hugeBin = BigInt("0b11111111111111111111111111111111111111111111111111111");
    console.log(hugeBin);
}
