//! Image blur: 16 passes of a 3x3 Gaussian blur over a W x W 8-bit image, ping-ponged between two
//! buffers. Flat byte arrays, nine loads per pixel at computed indices (`y * w + x`).
//!
//! Adapted from the Goose suite's `bench/rust/blur_index.rs` ("rust flat indexing"), the row that
//! indexes like the Fa version: the indices are a product of loop counters, so the bounds checks
//! stay in the loop.

use std::env;

const DEFAULT_SIZE: i64 = 3000;
const PASSES: i64 = 16;

/// The xorshift64 generator of the Goose suite.
fn xs_next(x: u64) -> u64 {
	let mut v = x;
	v ^= v << 13;
	v ^= v >> 7;
	v ^= v << 17;
	v
}

/// Reduction into [0, k), as C's uint64_t modulo; the result fits an i64.
fn xs_mod(x: u64, k: i64) -> i64 {
	(x % k as u64) as i64
}

fn blur(src: &[u8], dst: &mut [u8], w: usize) {
	for y in 1..w - 1 {
		for x in 1..w - 1 {
			let i = y * w + x;
			let s = src[i - w - 1] as u16
				+ src[i - w] as u16 * 2
				+ src[i - w + 1] as u16
				+ src[i - 1] as u16 * 2
				+ src[i] as u16 * 4
				+ src[i + 1] as u16 * 2
				+ src[i + w - 1] as u16
				+ src[i + w] as u16 * 2
				+ src[i + w + 1] as u16;
			dst[i] = (s >> 4) as u8;
		}
	}
}

/// Reads the size from the first command-line argument, or returns the default.
fn size_argument(default: i64) -> i64 {
	match env::args().nth(1) {
		Some(text) => text.parse().expect("the size must be an integer"),
		None => default,
	}
}

fn main() {
	let width = size_argument(DEFAULT_SIZE);
	let w = width as usize;
	let mut a: Vec<u8> = Vec::new();
	let mut b: Vec<u8> = Vec::new();
	let mut r: u64 = 12345;
	for _ in 0..width * width {
		r = xs_next(r);
		a.push(xs_mod(r, 256) as u8);
		b.push(0);
	}
	// Border pixels are never written, so b's border stays zero throughout.
	for _ in 0..PASSES / 2 {
		blur(&a, &mut b, w);
		blur(&b, &mut a, w);
	}
	let mut total: i64 = 0;
	for v in &a {
		total += *v as i64;
	}
	println!("{width}");
	println!("{total}");
}
