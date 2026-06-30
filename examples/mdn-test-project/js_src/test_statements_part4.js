// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 10 (fragment 32-41)
// Generated: 2026-06-30

function test_statements_part4() {
// ---- fragment 32 ----
try {{
        if (condition);      // Caution, this "if" does nothing!
          killTheUniverse(); // So this always gets executed!!!
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 32 error: ${e.message}`);
    }}

// ---- fragment 33 ----
try {{
        myModule.doAllTheAmazingThings();
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 33 error: ${e.message}`);
    }}

// ---- fragment 34 ----
try {{
        myModule.doAllTheAmazingThings(); // myModule.doAllTheAmazingThings is imported by the next line
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 34 error: ${e.message}`);
    }}

// ---- fragment 35 ----
try {{
        // getPrimes.js
        /**
         * Returns a list of prime numbers that are smaller than `max`.
         */
        function getPrimes(max) {
          const isPrime = Array.from({ length: max }, () => true);
          isPrime[0] = isPrime[1] = false;
          isPrime[2] = true;
          for (let i = 2; i * i < max; i++) {
            if (isPrime[i]) {
              for (let j = i ** 2; j < max; j += i) {
                isPrime[j] = false;
              }
            }
          }
          return [...isPrime.entries()]
            .filter(([, isPrime]) => isPrime)
            .map(([number]) => number);
        }
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 35 error: ${e.message}`);
    }}

// ---- fragment 36 ----
try {{
        console.log(getPrimes(10)); // [2, 3, 5, 7]
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 36 error: ${e.message}`);
    }}

// ---- fragment 37 ----
try {{
        // my-module.js
        let myValue = 1;
        setTimeout(() => {
          myValue = 2;
        }, 500);
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 37 error: ${e.message}`);
    }}

// ---- fragment 38 ----
try {{
        // main.js

        console.log(myValue); // 1
        console.log(myModule.myValue); // 1
        setTimeout(() => {
          console.log(myValue); // 2; my-module has updated its value
          console.log(myModule.myValue); // 2
          myValue = 3; // TypeError: Assignment to constant variable.
          // The importing module can only read the value but can't re-assign it.
        }, 1000);
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 38 error: ${e.message}`);
    }}

// ---- fragment 39 ----
try {{
        foo; // unqualified identifier
        foo.bar; // bar is a qualified identifier
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 39 error: ${e.message}`);
    }}

// ---- fragment 40 ----
try {{
        const foo = { bar: 1 };
        console.log(foo.bar);
        // foo is found in the scope chain as a variable;
        // bar is found in foo as a property
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 40 error: ${e.message}`);
    }}

// ---- fragment 41 ----
try {{
        console.log(globalThis.Math === Math); // true
    }} catch (e) {{
        console.error(`[test_statements_part4] fragment 41 error: ${e.message}`);
    }}

}
module.exports = { test_statements_part4 };
