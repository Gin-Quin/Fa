// Mandelbrot set on a size x size bitmap (Goose port of main.rs): float loops
// with an early exit, integer packing; prints a checksum instead of the image.

import std;
import os;

let ITERATIONS = 50;
let LIMIT = 4.0;
let MODULUS = 1000000007;

fn is_inside(cr: f64, ci: f64) -> bool {
    var zr = 0.0;
    var zi = 0.0;
    var tr = 0.0;
    var ti = 0.0;
    var iteration = 0;
    while iteration < ITERATIONS && tr + ti <= LIMIT {
        zi = 2.0 * zr * zi + ci;
        zr = tr - ti + cr;
        tr = zr * zr;
        ti = zi * zi;
        iteration++;
    }
    return tr + ti <= LIMIT;
}

fn main() {
    let a = args();
    var size = 4000;
    if a.len > 1 { let v, ok = parse_int(a[1]); if ok { size = v; } }
    var inside_count = 0;
    var checksum = 0;
    for y in size {
        let ci = 2.0 * (y as f64) / (size as f64) - 1.0;
        var byte = 0;
        var bit_count = 0;
        for x in size {
            let cr = 2.0 * (x as f64) / (size as f64) - 1.5;
            byte *= 2;
            if is_inside(cr, ci) {
                byte += 1;
                inside_count++;
            }
            bit_count++;
            if bit_count == 8 || x == size - 1 {
                // The last byte of a row is padded with zeros, as in the PBM format.
                while bit_count < 8 {
                    byte *= 2;
                    bit_count++;
                }
                checksum = (checksum * 31 + byte) % MODULUS;
                byte = 0;
                bit_count = 0;
            }
        }
    }
    print(size, "x", size, " pixels, ", inside_count, " inside");
    print("checksum: ", checksum);
}
