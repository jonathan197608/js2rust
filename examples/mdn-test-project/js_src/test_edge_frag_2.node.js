// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 2

function testEdge_frag_2() {
    try {
    console.log(0 === -0);
    console.log(NaN === NaN);
    console.log(null == undefined);
    console.log(null === undefined);
    console.log(0 == "");
    console.log(0 == "0");
    console.log(false == 0);
    console.log("" == false);
    } catch (e) {
        console.error(`[testEdge_frag_2] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_2();
}

module.exports = { testEdge_frag_2 };
