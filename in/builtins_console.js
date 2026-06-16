// builtins_console.js — Console method tests

function testConsoleLog() {
    console.log("hello");
    return 1;
}
function testConsoleError() {
    console.error("error!");
    return 2;
}
function testConsoleWarn() {
    console.warn("warning");
    return 3;
}

const test_testConsoleLog = testConsoleLog();
const test_testConsoleError = testConsoleError();
const test_testConsoleWarn = testConsoleWarn();
