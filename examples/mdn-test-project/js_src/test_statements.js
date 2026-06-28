// Auto-generated from MDN JS Reference
// Category: statements
// Fragments: 44
// Generated: 2026-06-28

function testStatements() {
    // ---- fragment 0 ----
    try {{
        function getRectArea(width, height) {
          if (width > 0 && height > 0) {
            return width * height;
          }
          return 0;
        }

        console.log(getRectArea(3, 4));

        console.log(getRectArea(-3, 4));
    }} catch (e) {{
        console.error(`[testStatements] fragment 0 error: ${e.message}`);
    }}

    // ---- fragment 1 ----
    try {{
        function counter() {
          // Infinite loop
          for (let count = 1; ; count++) {
            console.log(`${count}A`); // Until 5
            if (count === 5) {
              return;
            }
            console.log(`${count}B`); // Until 4
          }
          console.log(`${count}C`); // Never appears
        }

        counter();

        // Logs:
        // 1A
        // 1B
        // 2A
        // 2B
        // 3A
        // 3B
        // 4A
        // 4B
        // 5A
    }} catch (e) {{
        console.error(`[testStatements] fragment 1 error: ${e.message}`);
    }}

    // ---- fragment 2 ----
    try {{
        function magic() {
          return function calc(x) {
            return x * 42;
          };
        }

        const answer = magic();
        answer(1337); // 56154
    }} catch (e) {{
        console.error(`[testStatements] fragment 2 error: ${e.message}`);
    }}

    // ---- fragment 3 ----
    try {{
        let i = 0;

        while (i < 6) {
          if (i === 3) {
            break;
          }
          i += 1;
        }

        console.log(i);
    }} catch (e) {{
        console.error(`[testStatements] fragment 3 error: ${e.message}`);
    }}

    // ---- fragment 4 ----
    try {{
        function testBreak(x) {
          let i = 0;

          while (i < 6) {
            if (i === 3) {
              break;
            }
            i += 1;
          }

          return i * x;
        }
    }} catch (e) {{
        console.error(`[testStatements] fragment 4 error: ${e.message}`);
    }}

    // ---- fragment 5 ----
    try {{
        const food = "sushi";

        switch (food) {
          case "sushi":
            console.log("Sushi is originally from Japan.");
            break;
          case "pizza":
            console.log("Pizza is originally from Italy.");
            break;
          default:
            console.log("I have never heard of that dish.");
            break;
        }
    }} catch (e) {{
        console.error(`[testStatements] fragment 5 error: ${e.message}`);
    }}

    // ---- fragment 6 ----
    try {{
        outerBlock: {
          innerBlock: {
            console.log("1");
            break outerBlock; // breaks out of both innerBlock and outerBlock
            console.log(":-("); // skipped
          }
          console.log("2"); // skipped
        }
    }} catch (e) {{
        console.error(`[testStatements] fragment 6 error: ${e.message}`);
    }}

    // ---- fragment 7 ----
    try {{
        function getRectArea(width, height) {
          if (isNaN(width) || isNaN(height)) {
            throw new Error("Parameter is not a number!");
          }
        }

        try {
          getRectArea(3, "A");
        } catch (e) {
          console.error(e);
        }
    }} catch (e) {{
        console.error(`[testStatements] fragment 7 error: ${e.message}`);
    }}

    // ---- fragment 8 ----
    try {{
        throw expression;
    }} catch (e) {{
        console.error(`[testStatements] fragment 8 error: ${e.message}`);
    }}

    // ---- fragment 9 ----
    try {{
        throw error; // Throws a previously defined value (e.g. within a catch block)
        throw new Error("Required"); // Throws a new Error object
    }} catch (e) {{
        console.error(`[testStatements] fragment 9 error: ${e.message}`);
    }}

    // ---- fragment 10 ----
    try {{
        throw (
          new Error()
        );
    }} catch (e) {{
        console.error(`[testStatements] fragment 10 error: ${e.message}`);
    }}

    // ---- fragment 11 ----
    try {{
        function isNumeric(x) {
          return ["number", "bigint"].includes(typeof x);
        }

        function sum(...values) {
          if (!values.every(isNumeric)) {
            throw new TypeError("Can only add numbers");
          }
          return values.reduce((a, b) => a + b);
        }

        console.log(sum(1, 2, 3)); // 6
        try {
          sum("1", "2");
        } catch (e) {
          console.error(e); // TypeError: Can only add numbers
        }
    }} catch (e) {{
        console.error(`[testStatements] fragment 11 error: ${e.message}`);
    }}

    // ---- fragment 12 ----
    try {{
        readFile("foo.txt", (err, data) => {
          if (err) {
            throw err;
          }
          console.log(data);
        });
    }} catch (e) {{
        console.error(`[testStatements] fragment 12 error: ${e.message}`);
    }}

    // ---- fragment 13 ----
    try {{
        (async () => {{
            function readFilePromise(path) {
              return new Promise((resolve, reject) => {
                readFile(path, (err, data) => {
                  if (err) {
                    reject(err);
                  }
                  resolve(data);
                });
              });
            }

            try {
              const data = await readFilePromise("foo.txt");
              console.log(data);
            } catch (err) {
              console.error(err);
            }
        }})();
    }} catch (e) {{
        console.error(`[testStatements] fragment 13 error: ${e.message}`);
    }}

    // ---- fragment 14 ----
    try {{
        const number = 42;

        try {
          number = 99;
        } catch (err) {
          console.log(err);
          // (Note: the exact output may be browser-dependent)
        }

        console.log(number);
    }} catch (e) {{
        console.error(`[testStatements] fragment 14 error: ${e.message}`);
    }}

    // ---- fragment 15 ----
    try {{
        // define MY_FAV as a constant and give it the value 7
        const MY_FAV = 7;

        console.log(`my favorite number is: ${MY_FAV}`);
    }} catch (e) {{
        console.error(`[testStatements] fragment 15 error: ${e.message}`);
    }}

    // ---- fragment 16 ----
    try {{
        const MY_OBJECT = { key: "value" };
        MY_OBJECT = { OTHER_KEY: "value" };
    }} catch (e) {{
        console.error(`[testStatements] fragment 16 error: ${e.message}`);
    }}

    // ---- fragment 17 ----
    try {{
        MY_OBJECT.key = "otherValue";
    }} catch (e) {{
        console.error(`[testStatements] fragment 17 error: ${e.message}`);
    }}

    // ---- fragment 18 ----
    try {{
        const MY_ARRAY = [];
        MY_ARRAY = ["B"];
    }} catch (e) {{
        console.error(`[testStatements] fragment 18 error: ${e.message}`);
    }}

    // ---- fragment 19 ----
    try {{
        MY_ARRAY.push("A"); // ["A"]
    }} catch (e) {{
        console.error(`[testStatements] fragment 19 error: ${e.message}`);
    }}

    // ---- fragment 20 ----
    try {{
        const result = /(a+)(b+)(c+)/.exec("aaabcc");
        const [, a, b, c] = result;
        console.log(a, b, c); // "aaa" "b" "cc"
    }} catch (e) {{
        console.error(`[testStatements] fragment 20 error: ${e.message}`);
    }}

    // ---- fragment 21 ----
    try {{
        function calcRectArea(width, height) {
          return width * height;
        }

        console.log(calcRectArea(5, 6));
    }} catch (e) {{
        console.error(`[testStatements] fragment 21 error: ${e.message}`);
    }}

    // ---- fragment 22 ----
    try {{
        console.log(
          `'foo' name ${
            "foo" in globalThis ? "is" : "is not"
          } global. typeof foo is ${typeof foo}`,
        );
        if (false) {
          function foo() {
            return 1;
          }
        }

        // In Chrome:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Firefox:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Safari:
        // 'foo' name is global. typeof foo is function
    }} catch (e) {{
        console.error(`[testStatements] fragment 22 error: ${e.message}`);
    }}

    // ---- fragment 23 ----
    try {{
        console.log(
          `'foo' name ${
            "foo" in globalThis ? "is" : "is not"
          } global. typeof foo is ${typeof foo}`,
        );
        if (true) {
          function foo() {
            return 1;
          }
        }

        // In Chrome:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Firefox:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Safari:
        // 'foo' name is global. typeof foo is function
    }} catch (e) {{
        console.error(`[testStatements] fragment 23 error: ${e.message}`);
    }}

    // ---- fragment 24 ----
    try {{
        "use strict";

        {
          foo(); // Logs "foo"
          function foo() {
            console.log("foo");
          }
        }

        console.log(
          `'foo' name ${
            "foo" in globalThis ? "is" : "is not"
          } global. typeof foo is ${typeof foo}`,
        );
        // 'foo' name is not global. typeof foo is undefined
    }} catch (e) {{
        console.error(`[testStatements] fragment 24 error: ${e.message}`);
    }}

    // ---- fragment 25 ----
    try {{
        hoisted(); // Logs "foo"

        function hoisted() {
          console.log("foo");
        }
    }} catch (e) {{
        console.error(`[testStatements] fragment 25 error: ${e.message}`);
    }}

    // ---- fragment 26 ----
    try {{
        notHoisted(); // TypeError: notHoisted is not a function

        var notHoisted = function () {
          console.log("bar");
        };
    }} catch (e) {{
        console.error(`[testStatements] fragment 26 error: ${e.message}`);
    }}

    // ---- fragment 27 ----
    try {{
        function foo(a) {
          function a() {}
          console.log(typeof a);
        }

        foo(2); // Logs "function"
    }} catch (e) {{
        console.error(`[testStatements] fragment 27 error: ${e.message}`);
    }}

    // ---- fragment 28 ----
    try {{
        function calcSales(unitsA, unitsB, unitsC) {
          return unitsA * 79 + unitsB * 129 + unitsC * 699;
        }
    }} catch (e) {{
        console.error(`[testStatements] fragment 28 error: ${e.message}`);
    }}

    // ---- fragment 29 ----
    try {{
        const array = [1, 2, 3];

        // Assign all array values to 0
        for (let i = 0; i < array.length; array[i++] = 0 /* empty statement */);

        console.log(array);
    }} catch (e) {{
        console.error(`[testStatements] fragment 29 error: ${e.message}`);
    }}

    // ---- fragment 30 ----
    try {{
        ;
    }} catch (e) {{
        console.error(`[testStatements] fragment 30 error: ${e.message}`);
    }}

    // ---- fragment 31 ----
    try {{
        const arr = [1, 2, 3];

        // Assign all array values to 0
        for (let i = 0; i < arr.length; arr[i++] = 0) /* empty statement */ ;

        console.log(arr);
        // [0, 0, 0]
    }} catch (e) {{
        console.error(`[testStatements] fragment 31 error: ${e.message}`);
    }}

    // ---- fragment 32 ----
    try {{
        if (condition);      // Caution, this "if" does nothing!
          killTheUniverse(); // So this always gets executed!!!
    }} catch (e) {{
        console.error(`[testStatements] fragment 32 error: ${e.message}`);
    }}

    // ---- fragment 33 ----
    try {{
        myModule.doAllTheAmazingThings();
    }} catch (e) {{
        console.error(`[testStatements] fragment 33 error: ${e.message}`);
    }}

    // ---- fragment 34 ----
    try {{
        myModule.doAllTheAmazingThings(); // myModule.doAllTheAmazingThings is imported by the next line
    }} catch (e) {{
        console.error(`[testStatements] fragment 34 error: ${e.message}`);
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
        console.error(`[testStatements] fragment 35 error: ${e.message}`);
    }}

    // ---- fragment 36 ----
    try {{
        console.log(getPrimes(10)); // [2, 3, 5, 7]
    }} catch (e) {{
        console.error(`[testStatements] fragment 36 error: ${e.message}`);
    }}

    // ---- fragment 37 ----
    try {{
        // my-module.js
        let myValue = 1;
        setTimeout(() => {
          myValue = 2;
        }, 500);
    }} catch (e) {{
        console.error(`[testStatements] fragment 37 error: ${e.message}`);
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
        console.error(`[testStatements] fragment 38 error: ${e.message}`);
    }}

    // ---- fragment 39 ----
    try {{
        foo; // unqualified identifier
        foo.bar; // bar is a qualified identifier
    }} catch (e) {{
        console.error(`[testStatements] fragment 39 error: ${e.message}`);
    }}

    // ---- fragment 40 ----
    try {{
        const foo = { bar: 1 };
        console.log(foo.bar);
        // foo is found in the scope chain as a variable;
        // bar is found in foo as a property
    }} catch (e) {{
        console.error(`[testStatements] fragment 40 error: ${e.message}`);
    }}

    // ---- fragment 41 ----
    try {{
        console.log(globalThis.Math === Math); // true
    }} catch (e) {{
        console.error(`[testStatements] fragment 41 error: ${e.message}`);
    }}

    // ---- fragment 42 ----
    try {{
        let a, x, y;
        const r = 10;

        {
          const { PI, cos, sin } = Math;
          a = PI * r * r;
          x = r * cos(PI);
          y = r * sin(PI / 2);
        }
    }} catch (e) {{
        console.error(`[testStatements] fragment 42 error: ${e.message}`);
    }}

    // ---- fragment 43 ----
    try {{
        const objectHavingAnEspeciallyLengthyName = { foo: true, bar: false };

        if (((o) => o.foo && !o.bar)(objectHavingAnEspeciallyLengthyName)) {
          // This branch runs.
        }
    }} catch (e) {{
        console.error(`[testStatements] fragment 43 error: ${e.message}`);
    }}

}
module.exports = { testStatements };