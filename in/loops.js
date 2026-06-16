// Test: for-of loop translation

function sumArray(arr) {
    let total = 0;
    for (const x of arr) {
        total = total + x;
    }
    return total;
}

function maxValue(arr) {
    let max = arr[0];
    for (const n of arr) {
        if (n > max) {
            max = n;
        }
    }
    return max;
}

function countElements(arr) {
    let count = 0;
    for (const item of arr) {
        count = count + 1;
    }
    return count;
}

// for-in loops are not yet implemented (requires object key enumeration runtime)
// function findKeys(obj) {
//     let keys = "";
//     for (const k in obj) {
//         keys = keys + k;
//     }
//     return keys;
// }

// test_ variables for Zig test generation
const test_sumArray_loops = sumArray([1, 2, 3]);
const test_maxValue_loops = maxValue([5, 3, 8, 1]);
const test_countElements_loops = countElements([1, 2, 3, 4]);
