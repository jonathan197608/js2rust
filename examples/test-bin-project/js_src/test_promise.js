// test_promise.js — Promise 最小可用测试
// 覆盖：Promise.resolve, .then(), .catch()

export function testPromiseResolve() {
    // Promise.resolve(value) → fulfilled promise
    const p = Promise.resolve(42);
    p.then((v) => {
        console.log(v);
    });
    return 0;
}

export function testPromiseReject() {
    // Promise.reject(reason) → rejected promise
    const p = Promise.reject("oops");
    p.catch((err) => {
        console.log(err);
    });
    return 0;
}
