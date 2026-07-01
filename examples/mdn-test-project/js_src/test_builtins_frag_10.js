// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 10
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_10.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_10() {

        function isPrime(n) {
          if (n < 2n) {
            return false;
          }
          if (n % 2n === 0n) {
            return n === 2n;
          }
          for (let factor = 3n; factor * factor <= n; factor += 2n) {
            if (n % factor === 0n) {
              return false;
            }
          }
          return true;
        }

        // Takes a BigInt value as an argument, returns nth prime number as a BigInt value
        function nthPrime(nth) {
          let maybePrime = 2n;
          let prime = 0n;

          while (nth >= 0n) {
            if (isPrime(maybePrime)) {
              nth--;
              prime = maybePrime;
            }
            maybePrime++;
          }

          return prime;
        }

        nthPrime(20n);
        // 73n
    }
