// Safe Vec reference: no capacity hint; the same Park-Miller generator and checksums.
const MODULUS: i64 = 1_000_000_007;
const OPERATIONS: usize = 4_000_000;

struct Random {
	state: i64,
}

impl Random {
	fn next(&mut self, bound: i64) -> i64 {
		self.state = self.state * 48271 % 2147483647;
		self.state % bound
	}
}

fn main() {
	let n: i64 = std::env::args()
		.nth(1)
		.unwrap_or_else(|| "1000000".into())
		.parse()
		.unwrap();
	assert!(n > 0);
	let mut items = Vec::new();
	for i in 0..n {
		items.push(i);
	}
	let mut random = Random { state: 7 };
	let mut reads = 0;
	for _ in 0..OPERATIONS {
		let index = random.next(n) as usize;
		reads = (reads + items[index]) % MODULUS;
	}
	for _ in 0..OPERATIONS {
		let index = random.next(n) as usize;
		items[index] = random.next(1_000_000);
	}
	let mut writes = 0;
	for item in items {
		writes = (writes + item) % MODULUS;
	}
	println!("items: {n}, reads: {reads}, writes: {writes}");
}
