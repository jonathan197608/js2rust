// BUG-06: Array ES2023 methods (toReversed, toSorted, toSpliced, with) return
// ArrayList which cannot be indexed, iterated, or have .length accessed.
// - newArr[0] → ArrayList not indexable
// - newArr.length → utf16Len called on wrong type
// - for (const v of newArr) → ArrayList not iterable
// - const x = arr.toReversed(); x unused → unused local constant
// Status: BLOCKED by codegen bug. Enable when BUG-06 is fixed.

/** @returns {i64} */
export function testArrayWith() {
    const arr = [10, 20, 30, 40];
    const newArr = arr.with(1, 99);
    if (arr[1] === 20 && newArr[1] === 99 && newArr[0] === 10 && newArr[3] === 40) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testArrayWithNegativeIndex() {
    const arr = [10, 20, 30];
    const newArr = arr.with(-1, 99);
    if (arr[2] === 30 && newArr[2] === 99 && newArr[0] === 10) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testArrayToReversed() {
    const arr = [1, 2, 3, 4, 5];
    const reversed = arr.toReversed();
    if (arr[0] === 1 && reversed[0] === 5 && reversed[4] === 1) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testArrayToSorted() {
    const arr = [30, 10, 20];
    const sorted = arr.toSorted();
    if (arr[0] === 30 && sorted[0] === 10 && sorted[1] === 20 && sorted[2] === 30) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testArrayToSortedCustom() {
    const arr = [3, 1, 2];
    const sorted = arr.toSorted((a, b) => a - b);
    if (sorted[0] === 1 && sorted[1] === 2 && sorted[2] === 3) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testArrayToSpliced() {
    const arr = [10, 20, 30, 40, 50];
    const spliced = arr.toSpliced(1, 2, 99, 88);
    if (arr.length === 5 && spliced[0] === 10 && spliced[1] === 99 && spliced[2] === 88 && spliced[3] === 40 && spliced[4] === 50) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testArrayToReversedLength() {
    const arr = [1, 2, 3, 4, 5];
    const reversed = arr.toReversed();
    if (reversed.length === 5) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testArrayToSortedLength() {
    const arr = [30, 10, 20];
    const sorted = arr.toSorted();
    if (sorted.length === 3) {
        return 1;
    }
    return 0;
}
