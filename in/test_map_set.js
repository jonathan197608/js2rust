// Test Map functionality
function testMapBasic() {
    const map = new Map();
    map.set("a", 1);
    map.set("b", 2);
    const val = map.get("a");
    return val;
}

// Test Set functionality
function testSetBasic() {
    const set = new Set();
    set.add(1);
    set.add(2);
    const has = set.has(1);
    return has;
}
