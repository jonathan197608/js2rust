// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 211
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_211.node.js

function testBuiltins_frag_211() {
    try {
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
    } catch (e) {
        console.error(`[testBuiltins_frag_211] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_211();
}

module.exports = { testBuiltins_frag_211 };
