// Sieve of Eratosthenes (Goose port of main.rs): a large array of booleans,
// strided writes, a linear scan.

import std;
import os;

fn main() {
    let a = args();
    var limit = 100000000;
    if a.len > 1 { let v, ok = parse_int(a[1]); if ok { limit = v; } }
    var is_prime: bool[>..] = [];
    push_n(is_prime, true, limit + 1);

    var count = 0;
    var sum = 0;
    var largest = 0;
    for i in 2..is_prime.len {
        if is_prime[i] {
            count++;
            sum += i;
            largest = i;
            if i <= is_prime.len / i {
                var multiple = i * i;
                while multiple < is_prime.len {
                    is_prime[multiple] = false;
                    multiple += i;
                }
            }
        }
    }
    print("primes up to ", limit, ": ", count, ", sum: ", sum, ", largest: ", largest);
}
