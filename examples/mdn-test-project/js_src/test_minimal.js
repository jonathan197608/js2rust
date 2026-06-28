// test_minimal.js — 逐步增加测试（已验证到 #7）
export function testMinimal() {
    // ---- 1. 基础字符串 ----
    console.log("hello");

    // ---- 2. 数字变量 + 字符串拼接 ----
    let x = 10;
    console.log("x = " + x);

    // ---- 3. 算术 ----
    console.log("1+2 = " + (1 + 2));

    // ---- 4. 布尔值 ----
    console.log("true = " + true);

    // ---- 5. 比较运算符 ----
    console.log("3 > 2: " + (3 > 2));
    console.log("1 == 1: " + (1 == 1));

    // ---- 6. 字符串相等 ----
    console.log("'a' == 'a': " + ('a' == 'a'));

    // ---- 7. 逻辑运算符 ----
    console.log("true && false: " + (true && false));
    console.log("!true: " + (!true));

    // ---- 8. 数组 ----
    let arr = [1, 2, 3];
    console.log("arr.length = " + arr.length);

    // ---- 9. if 语句 ----
    if (x > 5) {
        console.log("x > 5");
    }

    // ---- 10. for 循环 ----
    for (let i = 0; i < 3; i = i + 1) {
        console.log("i = " + i);
    }

    // ---- 11. while 循环 ----
    let j = 0;
    while (j < 3) {
        console.log("j = " + j);
        j = j + 1;
    }

    // ---- 12. 字符串长度 ----
    let s = "hello";
    console.log("s.length = " + s.length);

    // ---- 13. Math 函数 ----
    console.log("Math.max(1,2,3) = " + Math.max(1, 2, 3));
    console.log("Math.min(1,2,3) = " + Math.min(1, 2, 3));
    console.log("Math.abs(-5) = " + Math.abs(-5));

    return 0;
}
