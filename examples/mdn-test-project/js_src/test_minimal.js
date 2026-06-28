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

    return 0;
}
