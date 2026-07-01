// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 130-139)
// Generated: 2026-06-28

function test_builtins_part14() {
// ---- fragment 130 ----
    try {{
        /a*/.exec("aaa"); // ['aaa']; the entire input is consumed
        /a*?/.exec("aaa"); // ['']; it's possible to consume no characters and still match successfully
        /^a*?$/.exec("aaa"); // ['aaa']; it's not possible to consume fewer characters and still match successfully
    }} catch (e) {{
        console.error(`[test_builtins_part14] fragment 130 error: ${e.message}`);
    }}

    
// ---- fragment 131 ----
    try {{
        /a*?$/.exec("aaa"); // ['aaa']; the match already succeeds at the first character, so the regex never attempts to start matching at the second character
    }} catch (e) {{
        console.error(`[test_builtins_part14] fragment 131 error: ${e.message}`);
    }}

    
// ---- fragment 132 ----
    try {{
        /[ab]+[abc]c/.exec("abbc"); // ['abbc']
    }} catch (e) {{
        console.error(`[test_builtins_part14] fragment 132 error: ${e.message}`);
    }}

    
// ---- fragment 133 ----
    try {{
        /(?=a)?b/.test("b"); // true; the lookahead is matched 0 time
    }} catch (e) {{
        console.error(`[test_builtins_part14] fragment 133 error: ${e.message}`);
    }}

    
// ---- fragment 134 ----
    try {{
        function countParagraphs(str) {
          return str.match(/(?:\r?\n){2,}/g).length + 1;
        }

        countParagraphs(`
        Paragraph 1

        Paragraph 2
        Containing some line breaks, but still the same paragraph

        Another paragraph
        `); // 3
    }} catch (e) {{
        console.error(`[test_builtins_part14] fragment 134 error: ${e.message}`);
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
        console.error(`[test_builtins_part14] fragment 135 error: ${e.message}`);
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
        console.error(`[test_builtins_part14] fragment 136 error: ${e.message}`);
    }}

    
// ---- fragment 137 ----
    try {{
        /\ba/.exec("abc");
        /c\b/.exec("abc");

        /\B /.exec(" abc");
        / \B/.exec("abc ");
    }} catch (e) {{
        console.error(`[test_builtins_part14] fragment 137 error: ${e.message}`);
    }}

    
// ---- fragment 138 ----
    try {{
        function hasThanks(str) {
          return /\b(thanks|thank you)\b/i.test(str);
        }

        hasThanks("Thanks! You helped me a lot."); // true
        hasThanks("Just want to say thank you for all your work."); // true
        hasThanks("Thanksgiving is around the corner."); // false
    }} catch (e) {{
        console.error(`[test_builtins_part14] fragment 138 error: ${e.message}`);
    }}

    
// ---- fragment 139 ----
    try {{
        var a = 2;
        try {
          throw new Error();
        } catch (a) {
          var a = 1; // This 1 is assigned to the caught `a`, not the outer `a`.
        }
        console.log(a); // 2

        try {
          throw new Error();
          // Note: identifier changed to `err` to avoid conflict with
          // the inner declaration of `a`.
        } catch (err) {
          var a = 1; // This 1 is assigned to the upper-scope `a`.
        }
        console.log(a); // 1
    }} catch (e) {{
        console.error(`[test_builtins_part14] fragment 139 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part14 };
