// Fannkuch-redux (Goose port of main.rs): small integer arrays,
// data-dependent indices, array copies.

import std;
import os;

fn fannkuch(n: i64) -> i64, i64 {
    var permutation: i64[>..] = [];
    for i in n { permutation.push(i); }
    var current: i64[>..] = [];
    push_n(current, 0, n);
    var count: i64[>..] = [];
    push_n(count, 0, n);
    var max_flips = 0;
    var checksum = 0;
    var permutation_index = 0;
    var r = n;
    loop {
        while r != 1 {
            count[r - 1] = r;
            r--;
        }

        copy_into(current, permutation);
        var flips = 0;
        var first = current[0];
        while first != 0 {
            reverse(current[..first + 1]);
            flips++;
            first = current[0];
        }
        max_flips = max(max_flips, flips);
        checksum += if permutation_index % 2 == 0 { flips } else { -flips };

        loop {
            if r == n { return checksum, max_flips; }
            // Rotate permutation[0..=r] left by one.
            let head = permutation[0];
            for i in r { permutation[i] = permutation[i + 1]; }
            permutation[r] = head;
            count[r] -= 1;
            if count[r] > 0 { break; }
            r++;
        }
        permutation_index++;
    }
}

fn main() {
    let a = args();
    var n = 10;
    if a.len > 1 { let v, ok = parse_int(a[1]); if ok { n = v; } }
    let checksum, max_flips = fannkuch(n);
    print(checksum);
    print("Pfannkuchen(", n, ") = ", max_flips);
}
