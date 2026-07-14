// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 10
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_10.node.js

function testBuiltins_frag_10() {
    try {
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

        console.log(nthPrime(20n));
    } catch (e) {
        console.error(`[testBuiltins_frag_10] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_10();
}

module.exports = { testBuiltins_frag_10 };
