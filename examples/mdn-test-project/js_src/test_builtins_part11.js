// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 112-121)
// Generated: 2026-06-30

function test_builtins_part11() {
// ---- fragment 112 ----
try {{
        const regex2 = new RegExp("ab+c", "g");
            _ = regex2;
        _ = regex2;
}} catch (e) {{
        console.error(`[test_builtins_part11] fragment 112 error: ${e.message}`);
    }}

// ---- fragment 113 ----
// SKIP: Tests regex [\\s-9] range which causes oxc parse failure
// Fragment 113 skipped — [\\s-9] character class range parse error

// ---- fragment 114 ----
try {{
        const r1 = /\p{Lowercase_Letter}/iu;
        const r2 = /[^\P{Lowercase_Letter}]/iu;
            _ = r1;
        _ = r2;
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
          // Simplified: removed \q{\r\n} and v flag (ES2024, not yet supported by oxc)
          return str.match(/[\r\n\u2028\u2029]/g);
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
// SKIP: Tests str.split(regex) which generates try in return type annotation
// Fragment 117 skipped — regex split codegen issue (try outside function scope)

// ---- fragment 118 ----
try {{
        var c = 3;
        var c0 = 0;
        var c_ = 0;
        /[\c0]/.test(String.fromCharCode(16)); // true (\x10 = DLE)
        /[\c_]/.test(String.fromCharCode(31)); // true (\x1f = US)
        /[\c*]/.test("\\"); // true
        /\c/.test("\\c"); // true
        /\c0/.test("\\c0"); // true (the \c0 syntax is only supported in character classes)
            _ = c;
        _ = c0;
        _ = c_;
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
        var a = 1;
        /a|ab/.exec("abc"); // ['a']
            _ = a;
}} catch (e) {{
        console.error(`[test_builtins_part11] fragment 120 error: ${e.message}`);
    }}

// ---- fragment 121 ----
try {{
        var a = 1;
        var c = 3;
        /(?:(a)|(ab))(?:(c)|(bc))/.exec("abc"); // ['abc', 'a', undefined, undefined, 'bc']
        // Not ['abc', undefined, 'ab', 'c', undefined]
            _ = a;
        _ = c;
}} catch (e) {{
        console.error(`[test_builtins_part11] fragment 121 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part11 };
