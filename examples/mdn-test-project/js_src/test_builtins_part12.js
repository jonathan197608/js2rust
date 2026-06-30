// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 122-131)
// Generated: 2026-06-30

function test_builtins_part12() {
// ---- fragment 122 ----
try {{
        function isImage(filename) {
          return /\.(?:png|jpe?g|webp|avif|gif)$/i.test(filename);
        }

        isImage("image.png"); // true
        isImage("image.jpg"); // true
        isImage("image.pdf"); // false
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 122 error: ${e.message}`);
    }}

// ---- fragment 123 ----
try {{
        function removeTrailingSlash(url) {
          return url.replace(/\/$/, "");
        }

        removeTrailingSlash("https://example.com/"); // "https://example.com"
        removeTrailingSlash("https://example.com/docs/"); // "https://example.com/docs"
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 123 error: ${e.message}`);
    }}

// ---- fragment 124 ----
try {{
        function isImage(filename) {
          return /\.(?:png|jpe?g|webp|avif|gif)$/i.test(filename);
        }

        isImage("image.png"); // true
        isImage("image.jpg"); // true
        isImage("image.pdf"); // false
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 124 error: ${e.message}`);
    }}

// ---- fragment 125 ----
try {{
        function isValidIdentifier(str) {
          return /^[$_\p{ID_Start}][$_\p{ID_Continue}]*$/u.test(str);
        }

        isValidIdentifier("foo"); // true
        isValidIdentifier("$1"); // true
        isValidIdentifier("1foo"); // false
        isValidIdentifier("  foo  "); // false
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 125 error: ${e.message}`);
    }}

// ---- fragment 126 ----
try {{
        const variables = ["foo", "foo:bar", "  foo  "];

        function toAssignment(key) {
          if (isValidIdentifier(key)) {
            return `globalThis.${key} = undefined;`;
          }
          // JSON.stringify() escapes quotes and other special characters
          return `globalThis[${JSON.stringify(key)}] = undefined;`;
        }

        const statements = variables.map(toAssignment).join("\n");

        console.log(statements);
        // globalThis.foo = undefined;
        // globalThis["foo:bar"] = undefined;
        // globalThis["  foo  "] = undefined;
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 126 error: ${e.message}`);
    }}

// ---- fragment 127 ----
try {{
        /\k/.test("k"); // true
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 127 error: ${e.message}`);
    }}

// ---- fragment 128 ----
try {{
        const re = /a{1, 3}/;
        re.test("aa"); // false
        re.test("a{1, 3}"); // true
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 128 error: ${e.message}`);
    }}

// ---- fragment 129 ----
try {{
        /[ab]*/.exec("aba"); // ['aba']
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 129 error: ${e.message}`);
    }}

// ---- fragment 130 ----
try {{
        /a*/.exec("aaa"); // ['aaa']; the entire input is consumed
        /a*?/.exec("aaa"); // ['']; it's possible to consume no characters and still match successfully
        /^a*?$/.exec("aaa"); // ['aaa']; it's not possible to consume fewer characters and still match successfully
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 130 error: ${e.message}`);
    }}

// ---- fragment 131 ----
try {{
        /a*?$/.exec("aaa"); // ['aaa']; the match already succeeds at the first character, so the regex never attempts to start matching at the second character
    }} catch (e) {{
        console.error(`[test_builtins_part12] fragment 131 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part12 };
