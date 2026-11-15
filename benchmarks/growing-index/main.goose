// Stable growable arrays, with no announced capacity. Same Park-Miller stream as Rust/Fa.
import std;
import os;

let MODULUS = 1000000007;
let OPERATIONS = 4000000;
struct Random { state: i64 }

fn next(random: Random&, bound: i64) -> i64 {
    random.state = random.state * 48271 % 2147483647;
    return random.state % bound;
}

fn main() {
    let a = args();
    var n = 1000000;
    if a.len > 1 { let value, ok = parse_int(a[1]); if ok { n = value; } }
    assert(n > 0);
    var items: i64[>..] = [];
    for i in n { items.push(i); }
    var random = Random { 7 };
    var reads = 0;
    for _ in OPERATIONS {
        let index = random.next(n);
        reads = (reads + items[index]) % MODULUS;
    }
    for _ in OPERATIONS {
        let index = random.next(n);
        items[index] = random.next(1000000);
    }
    var writes = 0;
    for item in items { writes = (writes + item) % MODULUS; }
    print("items: ", n, ", reads: ", reads, ", writes: ", writes);
}
