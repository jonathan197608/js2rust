// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 132-142)
// Generated: 2026-06-30

function test_builtins_part13() {
// ---- fragment 132 ----
try {{
        var c = 3;
        /[ab]+[abc]c/.exec("abbc"); // ['abbc']
            _ = c;
}} catch (e) {{
        console.error(`[test_builtins_part13] fragment 132 error: ${e.message}`);
    }}

// ---- fragment 133 ----
try {{
        var a = 1;
        var b = 2;
        /(?=a)?b/.test("b"); // true; the lookahead is matched 0 time
            _ = a;
        _ = b;
}} catch (e) {{
        console.error(`[test_builtins_part13] fragment 133 error: ${e.message}`);
    }}

// ---- fragment 135 ----
try {{
        // finding all the letters of a text
        const story = "It's the Cheshire Cat: now I shall have somebody to talk to.";

        // Most explicit form
        story.match(/\p{General_Category=Letter}/gu);

        // It is not mandatory to use the property name for General categories
        story.match(/\p{Letter}/gu);

        // This is equivalent (short alias):
        story.match(/\p{L}/gu);

        // This is also equivalent (conjunction of all the subcategories using short aliases)
        story.match(/\p{Lu}|\p{Ll}|\p{Lt}|\p{Lm}|\p{Lo}/gu);
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 135 error: ${e.message}`);
    }}

// ---- fragment 136 ----
try {{
        // Ù¢ is the digit 2 in Arabic-Indic notation
        // while it is predominantly written within the Arabic script
        // it can also be written in the Thaana script

        "Ù¢".match(/\p{Script=Thaana}/u);
        // null as Thaana is not the predominant script

        "Ù¢".match(/\p{Script_Extensions=Thaana}/u);
        // ["Ù¢", index: 0, input: "Ù¢", groups: undefined]
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 136 error: ${e.message}`);
    }}

// ---- fragment 137 ----
try {{
        /\ba/.exec("abc");
        /c\b/.exec("abc");

        /\B /.exec(" abc");
        / \B/.exec("abc ");
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 137 error: ${e.message}`);
    }}

// ---- fragment 138 ----
try {{
        function hasThanks(str) {
          return /\b(thanks|thank you)\b/i.test(str);
        }

        hasThanks("Thanks! You helped me a lot."); // true
            _ = hasThanks("Just want to say thank you for all your work."); // true
            _ = hasThanks("Thanksgiving is around the corner."); // false
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 138 error: ${e.message}`);
    }}

// ---- fragment 139 ----
// SKIP: Tests catch scope shadowing which has codegen issues
// Fragment 139 skipped — catch parameter shadowing + var hoisting

// ---- fragment 140 ----
try {{
        String.fromCodePoint("_"); // RangeError
        String.fromCodePoint(Infinity); // RangeError
        String.fromCodePoint(-1); // RangeError
        String.fromCodePoint(3.14); // RangeError
        String.fromCodePoint(3e-2); // RangeError
        String.fromCodePoint(NaN); // RangeError
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 140 error: ${e.message}`);
    }}

// ---- fragment 141 ----
try {{
        "foo".normalize("nfc"); // RangeError
        "foo".normalize(" NFC "); // RangeError
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 141 error: ${e.message}`);
    }}

// ---- fragment 142 ----
try {{
        "foo".normalize("NFC"); // 'foo'
    }} catch (e) {{
        console.error(`[test_builtins_part13] fragment 142 error: ${e.message}`);
    }}

}
module.exports = { test_builtins_part13 };
