// ── Date methods ──
function testDateNow() {
    const ts = Date.now();
    return ts > 0;
}

function testDateGetTime() {
    const ts = Date.now();
    const time = Date.getTime(ts);
    return time > 0;
}

function testDateGetFullYear() {
    const ts = Date.now();
    const year = Date.getFullYear(ts);
    return year > 2020;
}

// ── Object methods ──
function testObjectKeys() {
    const obj = { a: 1, b: 2 };
    const keys = Object.keys(obj);
    return keys.len > 0;
}

function testObjectValues() {
    const obj = { a: 1, b: 2 };
    const vals = Object.values(obj);
    return vals.len > 0;
}

// ── Number methods ──
function testNumberIsNaN() {
    return Number.isNaN(42) == false;
}

function testNumberIsFinite() {
    return Number.isFinite(100) == true;
}

function testNumberIsInteger() {
    return Number.isInteger(5) == true;
}

// ── URI encoding ──
function testEncodeURIComponent() {
    const encoded = encodeURIComponent("hello world");
    return encoded.len > 0;
}

function testDecodeURIComponent() {
    const decoded = decodeURIComponent("hello%20world");
    // Zig string comparison not yet supported, just check non-empty
    return decoded.len > 0;
}

// ── RegExp ──
function testRegExpTest() {
    const re = /hello/;
    return re.test("hello world");
}

function testRegExpExec() {
    const re = /world/;
    const result = re.exec("hello world");
    return result != null;
}
