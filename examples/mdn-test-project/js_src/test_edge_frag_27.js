// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 27
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_27() {
    function tryParse(str) {
        try {
            const result = JSON.parse(str);
            return "ok: " + result;
        } catch (e) {
            return "error: " + e.name;
        }
    }
    console.log(tryParse('{"valid": true}'));
    console.log(tryParse('invalid json'));
    try {
        throw new TypeError("custom error");
    } catch (e) {
        console.log(e instanceof TypeError);
        console.log(e instanceof Error);
        console.log(e.message);
        console.log(e.name);
    }
}
