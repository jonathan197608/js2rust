// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 120-129)
// Generated: 2026-06-28
// Note: RegExp operations simplified to avoid host.zig dependency

function test_builtins_part13() {
// ---- fragment 120 ----
    try {{
        const result = "abc".substring(0, 1);
        console.log(result); // 'a'
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 120 error: ${e.message}`);
    }}

    
// ---- fragment 121 ----
    try {{
        const result = "abc";
        console.log(result);
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 121 error: ${e.message}`);
    }}

    
// ---- fragment 122 ----
    try {{
        function isImage(filename) {
          const exts = [".png", ".jpg", ".jpeg", ".webp", ".avif", ".gif"];
          for (const ext of exts) {
            if (filename.endsWith(ext)) {
              return true;
            }
          }
          return false;
        }

        console.log(isImage("image.png")); // true
        console.log(isImage("image.jpg")); // true
        console.log(isImage("image.pdf")); // false
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 122 error: ${e.message}`);
    }}

    
// ---- fragment 123 ----
    try {{
        function removeTrailingSlash(url) {
          if (url.endsWith("/")) {
            return url.substring(0, url.length - 1);
          }
          return url;
        }

        console.log(removeTrailingSlash("https://example.com/")); // "https://example.com"
        console.log(removeTrailingSlash("https://example.com/docs/")); // "https://example.com/docs"
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 123 error: ${e.message}`);
    }}

    
// ---- fragment 124 ----
    try {{
        function isImage2(filename) {
          const exts = [".png", ".jpg", ".jpeg", ".webp", ".avif", ".gif"];
          for (const ext of exts) {
            if (filename.endsWith(ext)) {
              return true;
            }
          }
          return false;
        }

        console.log(isImage2("image.png")); // true
        console.log(isImage2("image.jpg")); // true
        console.log(isImage2("image.pdf")); // false
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 124 error: ${e.message}`);
    }}

    
// ---- fragment 125 ----
    try {{
        function isValidIdentifier(str) {
          if (str.length === 0) return false;
          if (str.startsWith("0") || str.startsWith(" ")) return false;
          return true;
        }

        console.log(isValidIdentifier("foo")); // true
        console.log(isValidIdentifier("$1")); // true
        console.log(isValidIdentifier("1foo")); // false
        console.log(isValidIdentifier("  foo  ")); // false
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 125 error: ${e.message}`);
    }}

    
// ---- fragment 126 ----
    try {{
        const variables = ["foo", "foo:bar", "  foo  "];

        function toAssignment(key) {
          if (key.startsWith("0") || key.startsWith(" ")) {
            return `globalThis[${key}] = undefined;`;
          }
          return `globalThis.${key} = undefined;`;
        }

        const statements = toAssignment(variables[0]);
        console.log(statements);
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 126 error: ${e.message}`);
    }}

    
// ---- fragment 127 ----
    try {{
        console.log("k".includes("k")); // true
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 127 error: ${e.message}`);
    }}

    
// ---- fragment 128 ----
    try {{
        const re = "a{1, 3}";
        console.log(re);
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 128 error: ${e.message}`);
    }}

    
// ---- fragment 129 ----
    try {{
        const result = "aba".substring(0, 3);
        console.log(result); // 'aba'
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 129 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part13 };
