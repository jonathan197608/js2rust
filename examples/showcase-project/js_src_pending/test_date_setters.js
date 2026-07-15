// BUG-07: Date setter methods lack optional parameter passing.
// JS: d.setFullYear(2025) → Zig expects setFullYear(self, year, month, date)
// with all 3 args. Missing args should be passed as null.
// Status: BLOCKED by codegen bug. Enable when BUG-07 is fixed.

/** @returns {i64} */
export function testDateSetFullYear() {
    const d = new Date(2020, 0, 1);
    d.setFullYear(2025);
    if (d.getFullYear() === 2025) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testDateSetMonth() {
    const d = new Date(2020, 0, 15);
    d.setMonth(5); // June (0-indexed)
    if (d.getMonth() === 5) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testDateSetDate() {
    const d = new Date(2020, 0, 1);
    d.setDate(20);
    if (d.getDate() === 20) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testDateSetHours() {
    const d = new Date(2020, 0, 1, 0, 0, 0);
    d.setHours(14);
    if (d.getHours() === 14) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testDateSetMinutes() {
    const d = new Date(2020, 0, 1, 12, 0, 0);
    d.setMinutes(30);
    if (d.getMinutes() === 30) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testDateSetSeconds() {
    const d = new Date(2020, 0, 1, 12, 0, 0);
    d.setSeconds(45);
    if (d.getSeconds() === 45) {
        return 1;
    }
    return 0;
}
