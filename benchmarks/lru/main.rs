//! LRU cache: a hash map from key to node, a doubly-linked recency list in an arena with a
//! free list, and a capacity that evicts the least recently used entry on a miss, under N
//! skewed lookups with an invalidation every 16th operation.
//!
//! Adapted from the Goose suite's `bench/rust/lru_hashmap.rs` (the "rust HashMap + arena"
//! row): `HashMap<i32, u32>` for the lookup, a `Vec<Node>` arena with `u32` links, two
//! sentinels and a free list for the recency list, like the Fa version (a `Map` and an
//! `Array` of nodes with integer links).

use std::collections::HashMap;
use std::env;

const DEFAULT_SIZE: i64 = 2000000;

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

struct Node {
	key: i32,
	val: i32,
	prev: u32,
	next: u32,
}

// Both sentinels live in the pool, so no link is ever null: the list is never empty.
const HEAD: usize = 0;
const TAIL: usize = 1;

fn unlink(pool: &mut [Node], n: u32) {
	let (p, q) = (pool[n as usize].prev, pool[n as usize].next);
	pool[p as usize].next = q;
	pool[q as usize].prev = p;
}

fn link_front(pool: &mut [Node], n: u32) {
	let q = pool[HEAD].next;
	pool[n as usize].prev = HEAD as u32;
	pool[n as usize].next = q;
	pool[q as usize].prev = n;
	pool[HEAD].next = n;
}

/// A freed slot comes back on the next allocation, so the pool never exceeds CAP + 2.
fn alloc(pool: &mut Vec<Node>, free: &mut Vec<u32>, key: i32, val: i32) -> u32 {
	let n = Node {
		key,
		val,
		prev: 0,
		next: 0,
	};
	match free.pop() {
		Some(i) => {
			pool[i as usize] = n;
			i
		}
		None => {
			pool.push(n);
			pool.len() as u32 - 1
		}
	}
}

fn main() {
	let n = size_argument(DEFAULT_SIZE);
	let capacity = n / 8;
	let keys = capacity * 2;
	let mut pool: Vec<Node> = Vec::new();
	pool.push(Node {
		key: -1,
		val: 0,
		prev: 0,
		next: TAIL as u32,
	});
	pool.push(Node {
		key: -1,
		val: 0,
		prev: HEAD as u32,
		next: 0,
	});
	let mut free: Vec<u32> = Vec::new();
	let mut map: HashMap<i32, u32> = HashMap::new();
	let mut count: i64 = 0;
	let mut hits: i64 = 0;
	let mut misses: i64 = 0;
	let mut removes: i64 = 0;
	let mut r: u64 = 12345;
	for _ in 0..n {
		r = xs_next(r);
		let a = xs_mod(r, keys);
		let key = (a * a) / keys; // Skewed towards small keys.
		let k = key as i32;
		let found = map.get(&k).copied();
		if xs_mod(r >> 40, 16) == 0 {
			// An invalidation: drop the entry if it is cached.
			if let Some(i) = found {
				unlink(&mut pool, i);
				map.remove(&k);
				free.push(i);
				count -= 1;
				removes += 1;
			}
			continue;
		}
		if let Some(i) = found {
			pool[i as usize].val += 1;
			unlink(&mut pool, i);
			link_front(&mut pool, i);
			hits += 1;
			continue;
		}
		misses += 1;
		if count == capacity {
			let lru = pool[TAIL].prev;
			unlink(&mut pool, lru);
			map.remove(&pool[lru as usize].key);
			free.push(lru);
			count -= 1;
		}
		let ni = alloc(&mut pool, &mut free, k, (key % 1000) as i32);
		map.insert(k, ni);
		link_front(&mut pool, ni);
		count += 1;
	}
	// Walk the list from most to least recent, so that the order is checked too.
	let mut walk: u64 = 0;
	let mut vals: i64 = 0;
	let mut cur = pool[HEAD].next as usize;
	while pool[cur].key >= 0 {
		walk = walk.wrapping_mul(31).wrapping_add(pool[cur].key as u64);
		vals += pool[cur].val as i64;
		cur = pool[cur].next as usize;
	}
	println!("{hits}");
	println!("{misses}");
	println!("{removes}");
	println!("{count}");
	println!("{walk}");
	println!("{vals}");
}
