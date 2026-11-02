// Recursive Fibonacci: function calls and integer additions (Goose port of main.rs).

import std;
import os;

recursive fn fib(n: i64) -> i64 {
    if n < 2 { return n; }
    return fib(n - 1) + fib(n - 2);
}

fn main() {
    let a = args();
    var n = 42;
    if a.len > 1 { let v, ok = parse_int(a[1]); if ok { n = v; } }
    print("fib(", n, ") = ", fib(n));
}
