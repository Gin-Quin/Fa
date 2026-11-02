//! JSON responses: for each of N requests, build a response object (an id, a user name, a
//! list of line items each with a string), render it as JSON with string escaping, checksum
//! the text with 64-bit FNV-1a, and throw both away.
//!
//! Adapted from the Goose suite's `bench/rust/respond_dto.rs` (the "rust DTO + String" row):
//! a struct with a `String` name and a `Vec<Item>` of items owning a `String` each, rendered
//! into a new `String`, like the Fa version (an object with a `String` and an `Array` of
//! objects, rendered into a `String`).

use std::env;
use std::fmt::Write;

const DEFAULT_SIZE: i64 = 400000;

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

struct Item {
	sku: String,
	qty: i64,
	price: i64,
}

struct Resp {
	id: i64,
	user: String,
	items: Vec<Item>,
}

/// A user name of 3 to 10 letters; one in eight carries a quote or a backslash, so that the
/// escaping path of the renderer is taken.
fn name_of(seed: u64) -> String {
	let mut s = String::new();
	let mut r = xs_next(seed);
	let n = 3 + xs_mod(r, 8);
	let odd = xs_mod(r >> 8, 8) == 0;
	for i in 0..n {
		r = xs_next(r);
		if odd && i == 1 {
			s.push(if xs_mod(r, 2) == 0 { '"' } else { '\\' });
		} else {
			s.push((b'a' + xs_mod(r, 26) as u8) as char);
		}
	}
	s
}

/// A stock keeping unit: two letters and four digits.
fn sku_of(seed: u64) -> String {
	let mut s = String::new();
	let mut r = xs_next(seed);
	s.push((b'A' + xs_mod(r, 26) as u8) as char);
	s.push((b'A' + xs_mod(r >> 8, 26) as u8) as char);
	for _ in 0..4 {
		r = xs_next(r);
		s.push((b'0' + xs_mod(r, 10) as u8) as char);
	}
	s
}

fn make_items(seed: u64, n: i64) -> Vec<Item> {
	let mut t: Vec<Item> = Vec::new();
	let mut r = seed;
	for _ in 0..n {
		r = xs_next(r);
		t.push(Item {
			sku: sku_of(r),
			qty: 1 + xs_mod(r >> 8, 5),
			price: 100 + xs_mod(r >> 16, 99900),
		});
	}
	t
}

// --- rendering ---------------------------------------------------------------

fn emit_str(out: &mut String, s: &str) {
	out.push('"');
	for c in s.chars() {
		if c == '"' || c == '\\' {
			out.push('\\');
		}
		out.push(c);
	}
	out.push('"');
}

fn render(r: &Resp) -> String {
	let mut out = String::new();
	write!(out, "{{\"id\":{}", r.id).unwrap();
	out.push_str(",\"user\":");
	emit_str(&mut out, &r.user);
	out.push_str(",\"items\":[");
	let mut total: i64 = 0;
	for (i, it) in r.items.iter().enumerate() {
		if i > 0 {
			out.push(',');
		}
		out.push_str("{\"sku\":");
		emit_str(&mut out, &it.sku);
		write!(out, ",\"qty\":{},\"price\":{}}}", it.qty, it.price).unwrap();
		total += it.qty * it.price;
	}
	write!(out, "],\"total\":{}}}", total).unwrap();
	out
}

/// 64-bit FNV-1a over the bytes of the text.
fn fnv(s: &[u8]) -> u64 {
	let mut h: u64 = 0xcbf29ce484222325;
	for &c in s {
		h = (h ^ c as u64).wrapping_mul(0x100000001b3);
	}
	h
}

/// One request: the response object and its rendering are both locals.
fn handle(seed: u64) -> (u64, i64) {
	let r = xs_next(seed);
	let n = 1 + xs_mod(r, 8);
	let resp = Resp {
		id: xs_mod(r >> 4, 1000000),
		user: name_of(r >> 8),
		items: make_items(r >> 16, n),
	};
	let out = render(&resp);
	(fnv(out.as_bytes()), out.len() as i64)
}

fn main() {
	let n = size_argument(DEFAULT_SIZE);
	let mut seed: u64 = 12345;
	let mut h: u64 = 0;
	let mut bytes: i64 = 0;
	for i in 0..n {
		seed = xs_next(seed);
		let (hh, l) = handle(seed);
		h ^= hh.wrapping_mul(31).wrapping_add(i as u64);
		bytes += l;
	}
	println!("{n}");
	println!("{bytes}");
	println!("{h}");
}
