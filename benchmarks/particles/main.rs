//! Particles: 200 steps of `f32` vector math over an array of structs, written elementwise
//! (`p.vel = p.vel + G`), with a bounce on the floor.
//!
//! Adapted from the Goose suite's `bench/rust/particles.rs` ("rust AoS, elementwise"): an array of
//! particles, each holding two three-component vectors added as whole values.
//!
//! All the arithmetic is exact in binary32 (positions are integers, velocities and gravity are
//! multiples of 1/16, and the only multiply is by 0.5), so every implementation must produce a
//! bit-identical checksum however the backend reassociates.

use std::env;
use std::ops::Add;

const DEFAULT_SIZE: i64 = 300_000;
const STEPS: i64 = 200;

#[derive(Clone, Copy)]
struct F3 {
	x: f32,
	y: f32,
	z: f32,
}

impl Add for F3 {
	type Output = F3;
	fn add(self, o: F3) -> F3 {
		F3 {
			x: self.x + o.x,
			y: self.y + o.y,
			z: self.z + o.z,
		}
	}
}

#[derive(Clone, Copy)]
struct P {
	pos: F3,
	vel: F3,
}

const G: F3 = F3 {
	x: 0.0,
	y: -0.0625,
	z: 0.0,
};

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

/// Reads the size from the first command-line argument, or returns the default.
fn size_argument(default: i64) -> i64 {
	match env::args().nth(1) {
		Some(text) => text.parse().expect("the size must be an integer"),
		None => default,
	}
}

fn main() {
	let n = size_argument(DEFAULT_SIZE);
	let mut r: u64 = 12345;
	let mut ps: Vec<P> = Vec::with_capacity(n as usize);
	for _ in 0..n {
		r = xs_next(r);
		ps.push(P {
			pos: F3 {
				x: xs_mod(r, 1024) as f32,
				y: xs_mod(r >> 12, 1024) as f32,
				z: xs_mod(r >> 24, 1024) as f32,
			},
			vel: F3 {
				x: (xs_mod(r >> 36, 256) - 128) as f32 / 16.0,
				y: (xs_mod(r >> 44, 256) - 128) as f32 / 16.0,
				z: (xs_mod(r >> 52, 256) - 128) as f32 / 16.0,
			},
		});
	}
	for _ in 0..STEPS {
		for p in ps.iter_mut() {
			p.vel = p.vel + G;
			p.pos = p.pos + p.vel;
			if p.pos.y < 0.0 {
				p.pos.y = -p.pos.y;
				p.vel.y = -(p.vel.y * 0.5);
			}
		}
	}
	let mut total: i64 = 0;
	for p in &ps {
		total += (p.pos.x * 16.0) as i64 + (p.pos.y * 16.0) as i64 + (p.pos.z * 16.0) as i64;
	}
	println!("{}", ps.len());
	println!("{total}");
}
