//! Expression-tree evaluation: a recursive enum with boxed children, built once, then evaluated
//! eight times with a `match` per node.
//!
//! Adapted from the Goose suite's `bench/rust/interp_box.rs` ("rust enum + Box"), the row whose
//! data structure matches the Fa version: a recursive union whose children are stored on the
//! heap, one allocation per node.

use std::env;

const DEFAULT_SIZE: i64 = 21;
const PASSES: i64 = 8;
/// Keeps every value in range: no overflow.
const M: i64 = 1000003;

enum Expr {
	Num(i32),
	Add(Box<Expr>, Box<Expr>),
	Mul(Box<Expr>, Box<Expr>),
	Neg(Box<Expr>),
}

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

fn build(count: &mut i64, depth: i64, seed: u64) -> Box<Expr> {
	let s = xs_next(seed);
	*count += 1;
	if depth == 0 {
		return Box::new(Expr::Num(xs_mod(s, 1000) as i32));
	}
	let k = xs_mod(s, 8);
	if k < 1 {
		return Box::new(Expr::Neg(build(count, depth - 1, s)));
	}
	let a = build(count, depth - 1, s);
	let b = build(count, depth - 1, xs_next(s));
	if k < 5 {
		Box::new(Expr::Add(a, b))
	} else {
		Box::new(Expr::Mul(a, b))
	}
}

/// The pass index enters at the leaves so the eight evaluations differ.
fn eval(e: &Expr, p: i64) -> i64 {
	match e {
		Expr::Num(v) => (*v as i64 + p) % M,
		Expr::Add(l, r) => (eval(l, p) + eval(r, p)) % M,
		Expr::Mul(l, r) => (eval(l, p) * eval(r, p)) % M,
		Expr::Neg(o) => (M - eval(o, p)) % M,
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
	let depth = size_argument(DEFAULT_SIZE);
	let mut count: i64 = 0;
	let root = build(&mut count, depth, 12345);
	let mut total: i64 = 0;
	for p in 0..PASSES {
		total = (total + eval(&root, p)) % M;
	}
	println!("{count}");
	println!("{total}");
}
