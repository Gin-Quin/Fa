// Spectral norm of an infinite matrix (Goose port of main.rs): nested loops
// over float arrays, integer-to-float conversions.

import std;
import os;
import math;

// The element (i, j) of the matrix A, indexed from 0.
fn element(i: i64, j: i64) -> f64 { 1.0 / (((i + j) * (i + j + 1) / 2 + i + 1) as f64) }

// result = A * vector
fn multiply(vector: f64[:], result: f64[:]) {
    for &slot, i in result {
        var sum = 0.0;
        for value, j in vector { sum += element(i, j) * value; }
        slot = sum;
    }
}

// result = transpose(A) * vector
fn multiply_transposed(vector: f64[:], result: f64[:]) {
    for &slot, i in result {
        var sum = 0.0;
        for value, j in vector { sum += element(j, i) * value; }
        slot = sum;
    }
}

// result = transpose(A) * A * vector
fn multiply_both(vector: f64[:], result: f64[:], temporary: f64[:]) {
    multiply(vector, temporary);
    multiply_transposed(temporary, result);
}

fn main() {
    let a = args();
    var n = 4000;
    if a.len > 1 { let v, ok = parse_int(a[1]); if ok { n = v; } }
    var u: f64[>..] = [];
    push_n(u, 1.0, n);
    var v: f64[>..] = [];
    push_n(v, 0.0, n);
    var temporary: f64[>..] = [];
    push_n(temporary, 0.0, n);
    for k in 10 {
        multiply_both(u, v, temporary);
        multiply_both(v, u, temporary);
    }
    var u_dot_v = 0.0;
    var v_dot_v = 0.0;
    for x, i in u {
        u_dot_v += x * v[i];
        v_dot_v += v[i] * v[i];
    }
    let norm = sqrt(u_dot_v / v_dot_v);
    print("spectral norm: ", round(norm * 1e9) as i64, " (x 1e-9)");
}
