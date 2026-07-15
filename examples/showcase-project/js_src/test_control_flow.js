// test_control_flow.js
// End-to-end tests for labeled statements.

// ── labeled statements ──

/** @returns {i64} */
export function testLabeledBreak() {
    let result = 0;
    outer: for (let i = 0; i < 3; i++) {
        for (let j = 0; j < 3; j++) {
            result = result + 1;
            if (i === 1 && j === 0) {
                break outer;
            }
        }
    }
    if (result === 4) {
        return 1;
    }
    return 0;
}
