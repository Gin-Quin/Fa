//! Mandelbrot set on a size x size bitmap: float loops with early exit, integer packing.
//! Prints a checksum of the packed bitmap instead of the image.

use std::env;

const DEFAULT_SIZE: i64 = 4_000;
const ITERATIONS: i64 = 50;
const LIMIT: f64 = 4.0;
const MODULUS: i64 = 1_000_000_007;

fn is_inside(cr: f64, ci: f64) -> bool {
	let mut zr = 0.0;
	let mut zi = 0.0;
	let mut tr = 0.0;
	let mut ti = 0.0;
	let mut iteration = 0;
	while iteration < ITERATIONS && tr + ti <= LIMIT {
		zi = 2.0 * zr * zi + ci;
		zr = tr - ti + cr;
		tr = zr * zr;
		ti = zi * zi;
		iteration += 1;
	}
	tr + ti <= LIMIT
}

/// Reads the size from the first command-line argument, or returns the default.
fn size_argument(default: i64) -> i64 {
	match env::args().nth(1) {
		Some(text) => text.parse().expect("the size must be an integer"),
		None => default,
	}
}

fn main() {
	let size = size_argument(DEFAULT_SIZE);
	let mut inside_count = 0;
	let mut checksum = 0;
	for y in 0..size {
		let ci = 2.0 * y as f64 / size as f64 - 1.0;
		let mut byte = 0;
		let mut bit_count = 0;
		for x in 0..size {
			let cr = 2.0 * x as f64 / size as f64 - 1.5;
			byte *= 2;
			if is_inside(cr, ci) {
				byte += 1;
				inside_count += 1;
			}
			bit_count += 1;
			if bit_count == 8 || x == size - 1 {
				// The last byte of a row is padded with zeros, as in the PBM format.
				while bit_count < 8 {
					byte *= 2;
					bit_count += 1;
				}
				checksum = (checksum * 31 + byte) % MODULUS;
				byte = 0;
				bit_count = 0;
			}
		}
	}
	println!("{size}x{size} pixels, {inside_count} inside");
	println!("checksum: {checksum}");
}
