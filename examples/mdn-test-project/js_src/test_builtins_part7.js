// Auto-generated from MDN JS Reference
// Category: builtins
// Fragments: 10 (fragment 60-69)
// Generated: 2026-06-28

function test_builtins_part7() {
// ---- fragment 60 ----
    try {{
        try {
          throw new AggregateError([new Error("some error")], "Hello");
        } catch (e) {
          console.log(e instanceof AggregateError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "AggregateError"
          console.log(e.errors); // [ Error: "some error" ]
        }
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 60 error: ${e.message}`);
    }}

    
// ---- fragment 61 ----
    try {{
        try {
          throw new EvalError("Hello");
        } catch (e) {
          console.log(e instanceof EvalError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "EvalError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 61 error: ${e.message}`);
    }}

    
// ---- fragment 62 ----
    try {{
        try {
          let a = undefinedVariable;
        } catch (e) {
          console.log(e instanceof ReferenceError); // true
          console.log(e.message); // "undefinedVariable is not defined"
          console.log(e.name); // "ReferenceError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 62 error: ${e.message}`);
    }}

    
// ---- fragment 63 ----
    try {{
        try {
          throw new ReferenceError("Hello");
        } catch (e) {
          console.log(e instanceof ReferenceError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "ReferenceError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 63 error: ${e.message}`);
    }}

    
// ---- fragment 64 ----
    try {{
        try {
          throw new SuppressedError(
            new Error("New error"),
            new Error("Original error"),
            "Hello",
          );
        } catch (e) {
          console.log(e instanceof SuppressedError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "SuppressedError"
          console.log(e.error); // Error: "New error"
          console.log(e.suppressed); // Error: "Original error"
        }
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 64 error: ${e.message}`);
    }}

    
// ---- fragment 65 ----
    try {{
        try {
          eval("hoo bar");
        } catch (e) {
          console.log(e instanceof SyntaxError); // true
          console.log(e.message);
          console.log(e.name); // "SyntaxError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 65 error: ${e.message}`);
    }}

    
// ---- fragment 66 ----
    try {{
        try {
          throw new SyntaxError("Hello");
        } catch (e) {
          console.log(e instanceof SyntaxError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "SyntaxError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 66 error: ${e.message}`);
    }}

    
// ---- fragment 67 ----
    try {{
        try {
          null.f();
        } catch (e) {
          console.log(e instanceof TypeError); // true
          console.log(e.message); // "null has no properties"
          console.log(e.name); // "TypeError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 67 error: ${e.message}`);
    }}

    
// ---- fragment 68 ----
    try {{
        try {
          throw new TypeError("Hello");
        } catch (e) {
          console.log(e instanceof TypeError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "TypeError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 68 error: ${e.message}`);
    }}

    
// ---- fragment 69 ----
    try {{
        try {
          decodeURIComponent("%");
        } catch (e) {
          console.log(e instanceof URIError); // true
          console.log(e.message); // "malformed URI sequence"
          console.log(e.name); // "URIError"
          console.log(e.stack); // Stack of the error
        }
    }} catch (e) {{
        console.error(`[test_builtins_part7] fragment 69 error: ${e.message}`);
    }}

    
}
module.exports = { test_builtins_part7 };
