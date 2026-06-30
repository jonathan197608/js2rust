// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 112-121)
// Generated: 2026-06-30

function test_builtins_part11() {
// ---- fragment 112 ----
try {{
        const regex2 = new RegExp("ab+c", "g");
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 112 error: ${e.message}`);
    }}

// ---- fragment 113 ----
try {{
        /[\s-9]/.test("-"); // true
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 113 error: ${e.message}`);
    }}

// ---- fragment 114 ----
try {{
        const r1 = /\p{Lowercase_Letter}/iu;
        const r2 = /[^\P{Lowercase_Letter}]/iu;
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 114 error: ${e.message}`);
    }}

// ---- fragment 115 ----
try {{
        function isHexadecimal(str) {
          return /^[0-9A-F]+$/i.test(str);
        }

        isHexadecimal("2F3"); // true
        isHexadecimal("beef"); // true
        isHexadecimal("undefined"); // false
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 115 error: ${e.message}`);
    }}

// ---- fragment 116 ----
try {{
        function getLineTerminators(str) {
          return str.match(/[\r\n\u2028\u2029\q{\r\n}]/gv);
        }

        getLineTerminators(`
        A poem\r
        Is split\r\n
        Into many
        Stanzas
        `); // [ '\r', '\r\n', '\n' ]
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 116 error: ${e.message}`);
    }}

// ---- fragment 117 ----
try {{
        function splitWords(str) {
          return str.split(/\s+/);
        }

        splitWords(`Look at the stars
        Look  how they\tshine for you`);
        // ['Look', 'at', 'the', 'stars', 'Look', 'how', 'they', 'shine', 'for', 'you']
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 117 error: ${e.message}`);
    }}

// ---- fragment 118 ----
try {{
        /[\c0]/.test("\x10"); // true
        /[\c_]/.test("\x1f"); // true
        /[\c*]/.test("\\"); // true
        /\c/.test("\\c"); // true
        /\c0/.test("\\c0"); // true (the \c0 syntax is only supported in character classes)
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 118 error: ${e.message}`);
    }}

// ---- fragment 119 ----
try {{
        const pattern = /a\nb/;
        const string = `a
        b`;
        console.log(pattern.test(string)); // true
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 119 error: ${e.message}`);
    }}

// ---- fragment 120 ----
try {{
        /a|ab/.exec("abc"); // ['a']
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 120 error: ${e.message}`);
    }}

// ---- fragment 121 ----
try {{
        /(?:(a)|(ab))(?:(c)|(bc))/.exec("abc"); // ['abc', 'a', undefined, undefined, 'bc']
        // Not ['abc', undefined, 'ab', 'c', undefined]
    }} catch (e) {{
        console.error(`[test_builtins_part11] fragment 121 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part11 };
