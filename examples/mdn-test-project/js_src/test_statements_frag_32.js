// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 32
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_32.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_32() {

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
    }
