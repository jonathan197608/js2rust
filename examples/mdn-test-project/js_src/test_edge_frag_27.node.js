// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 27

function testEdge_frag_27() {
    try {
    function tryParse(str) {
        try {
            const result = JSON.parse(str);
            return "ok: " + result;
        } catch (e) {
            return "error: " + e.message;
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
    } catch (e) {
        console.error(`[testEdge_frag_27] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_27();
}

module.exports = { testEdge_frag_27 };
