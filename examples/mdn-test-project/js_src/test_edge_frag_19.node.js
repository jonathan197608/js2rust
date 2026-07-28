// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 19

function testEdge_frag_19() {
    try {
    class Counter {
        constructor() {
            this.count = 0;
            this.name = "counter";
        }
        increment() { this.count++; return this; }
        value() { return this.count; }
        reset() { this.count = 0; return this; }
    }
    const c = new Counter();
    c.increment().increment().increment();
    console.log(c.value());
    c.reset();
    console.log(c.value());
    console.log(c.name);
    } catch (e) {
        console.error(`[testEdge_frag_19] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_19();
}

module.exports = { testEdge_frag_19 };
