//! Explicit open-addressed LRU table, adapted from Goose eadca40 bench/rust/lru_open.rs.
//! The standard HashMap row stays in main.rs. Read size from argv like the other Rust ports.
//! Keep the same 8-byte i32/u32 slot layout and zero empty index as Goose and open.fa;
//! arena indices 0 and 1 are sentinels, never cached entries. Only the empty marker changes
//! from upstream's signed -1; hashing, linear probes and backward-shift deletion stay intact.

use std::env;

const DEFAULT_SIZE: i64 = 2_000_000;

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
struct Slot {
	key: i32,
	idx: u32,
} // zero: empty (head is never a cached entry).

// Both sentinels live in the pool, so no link is ever null.
const HEAD: usize = 0;
const TAIL: usize = 1;

fn home(key: i32, mask: u64) -> usize {
	(((key as u64).wrapping_mul(0x9E3779B97F4A7C15) >> 24) & mask) as usize
}

// The slot holding `key`, or the empty slot where it would go.
fn find(slots: &[Slot], mask: u64, key: i32) -> usize {
	let mut i = home(key, mask);
	while slots[i].idx != 0 && slots[i].key != key {
		i = (i + 1) & mask as usize;
	}
	i
}

// Backward-shift deletion: slide later entries of the probe run down over the
// hole, so no tombstones accumulate.
fn map_remove(slots: &mut [Slot], mask: u64, at: usize) {
	let mut i = at;
	let mut j = i;
	loop {
		j = (j + 1) & mask as usize;
		let (jkey, jidx) = (slots[j].key, slots[j].idx);
		if jidx == 0 {
			break;
		}
		let h = home(jkey, mask);
		if (i < j && (h <= i || h > j)) || (i > j && h <= i && h > j) {
			slots[i] = Slot {
				key: jkey,
				idx: jidx,
			};
			i = j;
		}
	}
	slots[i].idx = 0;
}

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
	let mut nslots: i64 = 16;
	while nslots < capacity * 2 {
		nslots *= 2;
	}
	let mask = (nslots - 1) as u64;
	let mut slots: Vec<Slot> = (0..nslots).map(|_| Slot { key: 0, idx: 0 }).collect();
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
		let at = find(&slots, mask, k);
		let fi = slots[at].idx;
		if xs_mod(r >> 40, 16) == 0 {
			// An invalidation: drop the entry if it is cached.
			if fi != 0 {
				unlink(&mut pool, fi as u32);
				map_remove(&mut slots, mask, at);
				free.push(fi as u32);
				count -= 1;
				removes += 1;
			}
			continue;
		}
		if fi != 0 {
			pool[fi as usize].val += 1;
			unlink(&mut pool, fi as u32);
			link_front(&mut pool, fi as u32);
			hits += 1;
			continue;
		}
		misses += 1;
		let mut ins = at;
		if count == capacity {
			let lru = pool[TAIL].prev;
			unlink(&mut pool, lru);
			let lat = find(&slots, mask, pool[lru as usize].key);
			let li = slots[lat].idx;
			map_remove(&mut slots, mask, lat);
			free.push(li as u32);
			count -= 1;
			// The deletion may have shifted entries in this key's probe run,
			// so the empty slot found above can be stale.
			ins = find(&slots, mask, k);
		}
		let ni = alloc(&mut pool, &mut free, k, (key % 1000) as i32);
		slots[ins] = Slot { key: k, idx: ni };
		link_front(&mut pool, ni);
		count += 1;
	}
	// Walk the list from most to least recent, so the order is checked too.
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
	println!("{}", walk);
	println!("{vals}");
}
