//! S-expressions: generate N forms as text, parse them into an owning tree, walk the tree
//! four times.
//!
//! Adapted from the Goose suite's `bench/rust/sexp_box.rs` (the "rust enum + Box + String"
//! row): an `enum` whose variants own their children, with a `String` per symbol. The children
//! of a list are a `Vec<Node>` in source order, as in the Fa version, instead of the
//! first-child / previous-sibling `Option<Box<Node>>` links of the Goose row: the walk is a
//! sum, so the checksum doesn't depend on the order, and the output is the Goose program's.

use std::env;

const DEFAULT_SIZE: i64 = 20000;
const DEPTH: i64 = 4;
const PASSES: i64 = 4;

/// xorshift64, the generator of the Goose suite.
fn xs_next(x: u64) -> u64 {
	let mut v = x;
	v ^= v << 13;
	v ^= v >> 7;
	v ^= v << 17;
	v
}

/// Reduction into [0, k), as C's `uint64_t` modulo.
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

// --- generation --------------------------------------------------------------

fn emit_sym(text: &mut String, seed: u64) {
	let mut r = seed;
	let n = 3 + xs_mod(r, 12);
	for _ in 0..n {
		r = xs_next(r);
		text.push((b'a' + xs_mod(r, 26) as u8) as char);
	}
}

fn emit_num(text: &mut String, v: i64) {
	if v == 0 {
		text.push('0');
		return;
	}
	let mut tmp = [0u8; 24];
	let (mut k, mut d) = (0usize, v);
	while d > 0 {
		tmp[k] = b'0' + (d % 10) as u8;
		k += 1;
		d /= 10;
	}
	while k > 0 {
		k -= 1;
		text.push(tmp[k] as char);
	}
}

fn generate(text: &mut String, depth: i64, seed: u64) -> u64 {
	let mut r = xs_next(seed);
	if depth == 0 || xs_mod(r, 4) == 0 {
		if xs_mod(r >> 8, 2) == 0 {
			emit_sym(text, r);
		} else {
			emit_num(text, xs_mod(r >> 16, 100000));
		}
		text.push(' ');
		return r;
	}
	text.push('(');
	let kids = 2 + xs_mod(r >> 24, 4);
	for _ in 0..kids {
		r = generate(text, depth - 1, r);
	}
	text.push(')');
	text.push(' ');
	r
}

// --- parse and walk ----------------------------------------------------------

enum Node {
	Sym(String),
	Num(i32),
	List(Vec<Node>),
}

fn skip_space(b: &[u8], pos: &mut usize) {
	while *pos < b.len() && b[*pos] == b' ' {
		*pos += 1;
	}
}

fn parse_node(text: &str, pos: &mut usize, count: &mut i64) -> Node {
	let b = text.as_bytes();
	skip_space(b, pos);
	*count += 1;
	let c = b[*pos];
	if c == b'(' {
		*pos += 1;
		let mut items = Vec::new();
		loop {
			skip_space(b, pos);
			if b[*pos] == b')' {
				*pos += 1;
				break;
			}
			items.push(parse_node(text, pos, count));
		}
		return Node::List(items);
	}
	let start = *pos;
	if c.is_ascii_digit() {
		let mut v: i64 = 0;
		while *pos < b.len() && b[*pos].is_ascii_digit() {
			v = v * 10 + (b[*pos] - b'0') as i64;
			*pos += 1;
		}
		return Node::Num(v as i32);
	}
	while *pos < b.len() && b[*pos] != b' ' && b[*pos] != b'(' && b[*pos] != b')' {
		*pos += 1;
	}
	Node::Sym(text[start..*pos].to_string())
}

/// The pass index rides along so that the four walks are four different computations.
fn walk(n: &Node, p: i64) -> i64 {
	match n {
		Node::Sym(name) => name.len() as i64 + name.as_bytes()[0] as i64 + p,
		Node::Num(v) => (*v as i64) ^ p,
		Node::List(items) => {
			let mut total = 1;
			for item in items {
				total += walk(item, p);
			}
			total
		}
	}
}

fn main() {
	let n = size_argument(DEFAULT_SIZE);
	let mut text = String::new();
	let mut seed: u64 = 12345;
	for _ in 0..n {
		seed = generate(&mut text, DEPTH, seed);
	}
	let mut pos = 0usize;
	let mut count: i64 = 0;
	let mut roots: Vec<Node> = Vec::new();
	loop {
		skip_space(text.as_bytes(), &mut pos);
		if pos >= text.len() {
			break;
		}
		roots.push(parse_node(&text, &mut pos, &mut count));
	}
	println!("{}", text.len());
	println!("{count}");
	println!("{}", roots.len());
	println!("0");
	let mut total: i64 = 0;
	for p in 0..PASSES {
		for root in &roots {
			total += walk(root, p);
		}
	}
	println!("{total}");
}
