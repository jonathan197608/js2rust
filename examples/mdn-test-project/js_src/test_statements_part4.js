// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 10 (fragment 30-39)
// Generated: 2026-06-28

function test_statements_part4() {
// ---- fragment 30 ----
    try {{
        ;
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 30 error: ${e.message}`);
    }}

    
// ---- fragment 31 ----
    try {{
        const arr = [1, 2, 3];

        // Assign all array values to 0
        for (let i = 0; i < arr.length; arr[i++] = 0) /* empty statement */ ;

        console.log(arr);
        // [0, 0, 0]
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 31 error: ${e.message}`);
    }}

    
// ---- fragment 32 ----
    try {{
        const condition = false;
        function killTheUniverse() {}
        if (condition);      // Caution, this "if" does nothing!
          killTheUniverse(); // So this always gets executed!!!
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 32 error: ${e.message}`);
    }}

    
// ---- fragment 33 ----
    try {{
        const myModule = { myValue: 1 };
        console.log(myModule.myValue);
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 33 error: ${e.message}`);
    }}

    
// ---- fragment 34 ----
    try {{
        console.log("doAllTheAmazingThings");
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 34 error: ${e.message}`);
    }}

    
// ---- fragment 35 ----
    try {{
        let primeCount = 0;
        for (let i = 2; i < 10; i++) {
          let isPrime = true;
          for (let j = 2; j * j <= i; j++) {
            if (i % j === 0) {
              isPrime = false;
              break;
            }
          }
          if (isPrime) {
            primeCount += 1;
          }
        }
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 35 error: ${e.message}`);
    }}

    
// ---- fragment 36 ----
    try {{
        console.log("primes: 2, 3, 5, 7");
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 36 error: ${e.message}`);
    }}

    
// ---- fragment 37 ----
    try {{
        // my-module.js
        let myValue = 1;
        myValue = 2; // Simplified: setTimeout not supported
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 37 error: ${e.message}`);
    }}

    
// ---- fragment 38 ----
    try {{
        // main.js
        let myValue = 1;
        console.log(myValue); // 1
        console.log(myValue); // 1
        myValue = 3; // TypeError: Assignment to constant variable.
        // The importing module can only read the value but can't re-assign it.
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 38 error: ${e.message}`);
    }}

    
// ---- fragment 39 ----
    try {{
        const foo = { bar: 1 };
        console.log(foo); // unqualified identifier
        console.log(foo.bar); // bar is a qualified identifier
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 39 error: ${e.message}`);
    }}

    
}
module.exports = { test_statements_part4 };
