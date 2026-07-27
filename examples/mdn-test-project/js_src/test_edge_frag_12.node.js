// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 12

function testEdge_frag_12() {
    try {
    const emoji = "\uD83D\uDE00";
    console.log(emoji.length);
    console.log(emoji.charCodeAt(0));
    console.log(emoji.charCodeAt(1));
    console.log(emoji.codePointAt(0));
    console.log("hello".at(-1));
    console.log("hello".at(0));
    console.log("hello".at(10));
    console.log("".padStart(5, "ab"));
    } catch (e) {
        console.error(`[testEdge_frag_12] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_12();
}

module.exports = { testEdge_frag_12 };
