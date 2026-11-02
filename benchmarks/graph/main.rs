//! Graph traversal: adjacency lists grown one edge at a time (V vertices, 8V random edges), then
//! a breadth-first search from four sources over a reused queue and distance array.
//!
//! Adapted from the Goose suite's `bench/rust/graph_nested.rs` ("rust Vec<Vec>"), the row whose
//! data structure matches the Fa version: one growable array of neighbours per vertex.
//!
//! BFS distances are shortest paths, so the checksum does not depend on the order edges come out
//! of the adjacency structure.

use std::env;

const DEFAULT_SIZE: i64 = 100_000;
const DEG: i64 = 8;
const SOURCES: i64 = 4;

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
	let nv = size_argument(DEFAULT_SIZE);
	let ne = nv * DEG;
	let mut r: u64 = 12345;
	let mut adj: Vec<Vec<i32>> = (0..nv).map(|_| Vec::new()).collect();
	for _ in 0..ne {
		r = xs_next(r);
		let u = xs_mod(r, nv) as usize;
		adj[u].push(xs_mod(r >> 24, nv) as i32);
	}

	let mut dist: Vec<i32> = vec![-1; nv as usize];
	let mut q: Vec<i32> = Vec::with_capacity(nv as usize);
	let mut total: i64 = 0;
	let mut seed: u64 = 999;
	for _ in 0..SOURCES {
		seed = xs_next(seed);
		let src = xs_mod(seed, nv) as usize;
		for d in dist.iter_mut() {
			*d = -1;
		}
		q.clear();
		dist[src] = 0;
		q.push(src as i32);
		let mut qh = 0;
		while qh < q.len() {
			let u = q[qh] as usize;
			qh += 1;
			let d = dist[u];
			total += d as i64;
			// `adj` is borrowed immutably while `dist` and `q` are mutated:
			// separate locals, so the borrow checker allows the iterator form.
			for &e in &adj[u] {
				let w = e as usize;
				if dist[w] < 0 {
					dist[w] = d + 1;
					q.push(w as i32);
				}
			}
		}
	}
	println!("{}", nv + ne);
	println!("{total}");
}
