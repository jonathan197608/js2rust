// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 32
// Source: test_statements_part*.js
// Run: node test_statements_frag_32.node.js

function testStatements_frag_32() {
    try {

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
        } catch (e) {
        console.error(`[testStatements_frag_32] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_32();
}

module.exports = { testStatements_frag_32 };
