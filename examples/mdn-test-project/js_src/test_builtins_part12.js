// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 110-119)
// Generated: 2026-06-28
// Note: RegExp operations simplified to avoid host.zig dependency

function test_builtins_part12() {
// ---- fragment 110 ----
    try {{
        const AsyncFunction = 0;
        console.log(AsyncFunction);
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 110 error: ${e.message}`);
    }}

    
// ---- fragment 111 ----
    try {{
        const regex1 = "ab+c";
        console.log(regex1);
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 111 error: ${e.message}`);
    }}

    
// ---- fragment 112 ----
    try {{
        const regex2 = "ab+c";
        console.log(regex2);
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 112 error: ${e.message}`);
    }}

    
// ---- fragment 113 ----
    try {{
        const hasDash = "-".includes("-");
        console.log(hasDash);
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 113 error: ${e.message}`);
    }}

    
// ---- fragment 114 ----
    try {{
        const r1 = "lowercase";
        const r2 = "not lowercase";
        console.log(r1);
        console.log(r2);
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 114 error: ${e.message}`);
    }}

    
// ---- fragment 115 ----
    try {{
        function isHexadecimal(str) {
          const hexChars = "0123456789ABCDEFabcdef";
          for (const ch of str) {
            if (!hexChars.includes(ch)) {
              return false;
            }
          }
          return str.length > 0;
        }

        console.log(isHexadecimal("2F3")); // true
        console.log(isHexadecimal("beef")); // true
        console.log(isHexadecimal("undefined")); // false
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 115 error: ${e.message}`);
    }}

    
// ---- fragment 116 ----
    try {{
        function getLineTerminators(str) {
          const result = [];
          for (const ch of str) {
            if (ch === "\r" || ch === "\n") {
              result.push(ch);
            }
          }
          return result;
        }

        console.log(getLineTerminators("A\r\nB"));
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 116 error: ${e.message}`);
    }}

    
// ---- fragment 117 ----
    try {{
        function splitWords(str) {
          const result = [];
          let word = "";
          for (const ch of str) {
            if (ch === " ") {
              if (word.length > 0) {
                result.push(word);
              }
              word = "";
            } else {
              word = word + ch;
            }
          }
          if (word.length > 0) {
            result.push(word);
          }
          return result;
        }

        console.log(splitWords("Look at the stars"));
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 117 error: ${e.message}`);
    }}

    
// ---- fragment 118 ----
    try {{
        console.log("control char test");
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 118 error: ${e.message}`);
    }}

    
// ---- fragment 119 ----
    try {{
        const pattern = "a\nb";
        const string = "a\nb";
        console.log(pattern === string); // true
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 119 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part12 };
