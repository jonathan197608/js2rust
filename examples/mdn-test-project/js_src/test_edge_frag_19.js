// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 19
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_19() {
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
}
