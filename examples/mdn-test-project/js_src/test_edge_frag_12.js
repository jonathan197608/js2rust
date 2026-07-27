// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 12
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_12() {
    const emoji = "\uD83D\uDE00";
    console.log(emoji.length);
    console.log(emoji.charCodeAt(0));
    console.log(emoji.charCodeAt(1));
    console.log(emoji.codePointAt(0));
    console.log("hello".at(-1));
    console.log("hello".at(0));
    console.log("hello".at(10));
    console.log("".padStart(5, "ab"));
}
