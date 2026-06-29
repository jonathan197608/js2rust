// Auto-generated from MDN JS Reference
// Category: expressions
// Fragments: 10 (fragment 110-119)
// Generated: 2026-06-28

function test_expressions_part12() {
    let x = 5;
    let y = 3;
// ---- fragment 110 ----
    try {{
        // Before: 11100110111110100000000000000110000000000001
        // After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[test_expressions_part12] fragment 110 error: ${e.message}`);
    }}

    
// ---- fragment 111 ----
    try {{
        9 >> 2; // 2
        -9 >> 2; // -3

        9n >> 2n; // 2n
    }} catch (e) {{
        console.error(`[test_expressions_part12] fragment 111 error: ${e.message}`);
    }}

    
// ---- fragment 112 ----
    try {{
        const a = 5; //  00000000000000000000000000000101
        const b = 2; //  00000000000000000000000000000010
        const c = -5; //  11111111111111111111111111111011

        console.log(a >>> b); //  00000000000000000000000000000001

        console.log(c >>> b); //  00111111111111111111111111111110
    }} catch (e) {{
        console.error(`[test_expressions_part12] fragment 112 error: ${e.message}`);
    }}

    
// ---- fragment 113 ----
    try {{
        x >>> y
    }} catch (e) {{
        console.error(`[test_expressions_part12] fragment 113 error: ${e.message}`);
    }}

    
// ---- fragment 114 ----
    try {{
        // Before: 11100110111110100000000000000110000000000001
        // After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[test_expressions_part12] fragment 114 error: ${e.message}`);
    }}

    
// ---- fragment 115 ----
    try {{
        9 >>> 2; // 2
        -9 >>> 2; // 1073741821
    }} catch (e) {{
        console.error(`[test_expressions_part12] fragment 115 error: ${e.message}`);
    }}

    
// ---- fragment 116 ----
    try {{
        9n >>> 2n; // TypeError: BigInts have no unsigned right shift, use >> instead
    }} catch (e) {{
        console.error(`[test_expressions_part12] fragment 116 error: ${e.message}`);
    }}

    
// ---- fragment 117 ----
    try {{
        const a = 5; // 00000000000000000000000000000101
        const b = 3; // 00000000000000000000000000000011

        console.log(a & b); // 00000000000000000000000000000001
    }} catch (e) {{
        console.error(`[test_expressions_part12] fragment 117 error: ${e.message}`);
    }}

    
// ---- fragment 118 ----
    try {{
        x & y
    }} catch (e) {{
        console.error(`[test_expressions_part12] fragment 118 error: ${e.message}`);
    }}

    
// ---- fragment 119 ----
    try {{
        // Before: 11100110111110100000000000000110000000000001
        // After:              10100000000000000110000000000001
    }} catch (e) {{
        console.error(`[test_expressions_part12] fragment 119 error: ${e.message}`);
    }}

    
}
module.exports = { test_expressions_part12 };
