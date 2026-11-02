//! Scene graph: a tree of nodes with a variable number of children, each with a local transform;
//! for 16 frames, some nodes move, then the world transforms are recomputed top-down (3x3 `f32`
//! matrix products) and summed.
//!
//! Adapted from the Goose suite's `bench/rust/scene_box.rs` ("rust Box children"): every node owns
//! its children in a growable array, and the passes are recursive. The children are stored
//! directly in the vector (`Vec<SNode>` rather than `Vec<Box<SNode>>`), as Fa's
//! `Array(SceneNode)` stores them.
//!
//! Every transform is an axis-aligned 90-degree rotation with an integer translation, so all the
//! float math is exact and the checksum is bit-identical across languages.

use std::env;

const DEFAULT_SIZE: i64 = 12;
const FRAMES: i64 = 16;
const MAXKIDS: i64 = 4;

#[derive(Clone, Copy)]
struct F3 {
	x: f32,
	y: f32,
	z: f32,
}

#[derive(Clone, Copy)]
struct Xf {
	m: [f32; 9],
	t: F3,
}

struct SNode {
	id: i32,
	local: Xf,
	world: Xf,
	kids: Vec<SNode>,
}

const IDENT: Xf = Xf {
	m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
	t: F3 {
		x: 0.0,
		y: 0.0,
		z: 0.0,
	},
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

/// One of twelve axis-aligned rotations, from the seed.
fn rotation(seed: u64) -> [f32; 9] {
	let axis = xs_mod(seed, 3);
	let quarter = xs_mod(seed >> 8, 4);
	let mut c: f32 = 1.0;
	let mut s: f32 = 0.0;
	if quarter == 1 {
		c = 0.0;
		s = 1.0;
	} else if quarter == 2 {
		c = -1.0;
		s = 0.0;
	} else if quarter == 3 {
		c = 0.0;
		s = -1.0;
	}
	if axis == 0 {
		return [1.0, 0.0, 0.0, 0.0, c, 0.0 - s, 0.0, s, c];
	}
	if axis == 1 {
		return [c, 0.0, s, 0.0, 1.0, 0.0, 0.0 - s, 0.0, c];
	}
	[c, 0.0 - s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0]
}

/// The node's id is its creation index, assigned before its children are built, so the ids run
/// in pre-order.
fn build(count: &mut i64, depth: i64, seed: u64) -> SNode {
	let r = xs_next(seed);
	let id = *count as i32;
	*count += 1;
	let mut nd = SNode {
		id,
		local: Xf {
			m: rotation(r),
			t: F3 {
				x: xs_mod(r >> 16, 9) as f32,
				y: xs_mod(r >> 24, 9) as f32,
				z: xs_mod(r >> 32, 9) as f32,
			},
		},
		world: IDENT,
		kids: Vec::new(),
	};
	if depth == 0 {
		return nd;
	}
	let n = 1 + xs_mod(r >> 40, MAXKIDS);
	for i in 0..n {
		nd.kids.push(build(count, depth - 1, xs_next(r.wrapping_add(i as u64))));
	}
	nd
}

/// world = parent o local: a 3x3 product and a transformed translation.
fn compose(p: &Xf, l: &Xf, w: &mut Xf) {
	for i in 0..3 {
		for j in 0..3 {
			let mut s: f32 = 0.0;
			for k in 0..3 {
				s = s + p.m[i * 3 + k] * l.m[k * 3 + j];
			}
			w.m[i * 3 + j] = s;
		}
	}
	w.t.x = p.m[0] * l.t.x + p.m[1] * l.t.y + p.m[2] * l.t.z + p.t.x;
	w.t.y = p.m[3] * l.t.x + p.m[4] * l.t.y + p.m[5] * l.t.z + p.t.y;
	w.t.z = p.m[6] * l.t.x + p.m[7] * l.t.y + p.m[8] * l.t.z + p.t.z;
}

/// The parent's world transform is borrowed in place: `n.kids` and `n.world` are disjoint fields,
/// so the child may be mutated while the parent is read.
fn update(n: &mut SNode, parent: &Xf) {
	compose(parent, &n.local, &mut n.world);
	for k in &mut n.kids {
		update(k, &n.world);
	}
}

/// A seventh of the nodes move each frame: the animation.
fn animate(n: &mut SNode, f: i64, step: f32) {
	if n.id as i64 % 7 == f % 7 {
		n.local.t.x = step;
		n.local.t.z = 8.0 - step;
	}
	for k in &mut n.kids {
		animate(k, f, step);
	}
}

/// Where everything ended up, as an integer: exact in f32, summed in i64.
fn sum_world(n: &SNode) -> i64 {
	let mut t = (n.world.t.x + n.world.t.y + n.world.t.z) as i64;
	for k in &n.kids {
		t += sum_world(k);
	}
	t
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
	let mut root = build(&mut count, depth, 12345);
	let mut total: i64 = 0;
	for f in 0..FRAMES {
		let step = (f % 5) as f32;
		animate(&mut root, f, step);
		update(&mut root, &IDENT);
		total += sum_world(&root);
	}
	println!("{count}");
	println!("{total}");
}
