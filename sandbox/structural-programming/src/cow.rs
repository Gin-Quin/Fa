// mod A {
// 	pub struct Main {
// 		pub x: i64,
// 		pub y: Vec<u8>,
// 		pub z: bool,
// 		pub n1: i64,
// 		pub n2: i64,
// 		pub n3: i64,
// 		pub n4: i64,
// 		pub n5: i64,
// 		pub n6: i64,
// 		pub n7: i64,
// 	}

// 	pub fn log(main: *const Main) {
// 		println!("A: {:?}", (*main).x);
// 	}
// }

// mod B {
// 	pub struct Main {
// 		pub x: i64,
// 		pub y: Vec<u8>,
// 		pub n1: i64,
// 		pub n2: i64,
// 		pub n3: i64,
// 		pub n4: i64,
// 		pub n5: i64,
// 		pub n6: i64,
// 		pub n7: i64,
// 	}

// 	pub fn log(main: &Main) {
// 		println!("B: {:?}", main.x);
// 	}
// }

// fn call_b(b: B::Main) -> i64 {
// 	// B::log(&b);
// 	b.x
// }

// fn call_b_ref(b: &B::Main) -> i64 {
// 	// B::log(b);
// 	b.x
// }

// fn benchmark_struct() {
// 	use std::time::Instant;
// 	const ITERATIONS: u32 = 1_000_000;

// 	let a = A::Main {
// 		x: 42,
// 		y: vec![84],
// 		z: true,
// 		n1: 1,
// 		n2: 2,
// 		n3: 3,
// 		n4: 4,
// 		n5: 5,
// 		n6: 6,
// 		n7: 7,
// 	};
// 	let start = Instant::now();

// 	for _ in 0..ITERATIONS {
// 		call_b(B::Main {
// 			x: a.x,
// 			y: a.y,
// 			n1: a.n1,
// 			n2: a.n2,
// 			n3: a.n3,
// 			n4: a.n4,
// 			n5: a.n5,
// 			n6: a.n6,
// 			n7: a.n7,
// 		});
// 	}

// 	let duration = start.elapsed();
// 	println!("Time elapsed for {} iterations: {:?}", ITERATIONS, duration);
// 	println!("Average time per iteration: {:?}", duration / ITERATIONS);
// }

// fn benchmark_ref() {
// 	use std::time::Instant;
// 	const ITERATIONS: u32 = 1_000_000;

// 	let a = A::Main {
// 		x: 42,
// 		y: vec![84],
// 		z: true,
// 		n1: 1,
// 		n2: 2,
// 		n3: 3,
// 		n4: 4,
// 		n5: 5,
// 		n6: 6,
// 		n7: 7,
// 	};

// 	A::log(&a);

// 	let start = Instant::now();

// 	for _ in 0..ITERATIONS {
// 		call_b_ref(
// 			&(B::Main {
// 				x: a.x,
// 				y: &a.y,
// 				n1: a.n1,
// 				n2: a.n2,
// 				n3: a.n3,
// 				n4: a.n4,
// 				n5: a.n5,
// 				n6: a.n6,
// 				n7: a.n7,
// 			})
// 		);
// 	}

// 	let duration = start.elapsed();
// 	println!("Time elapsed for {} iterations: {:?}", ITERATIONS, duration);
// 	println!("Average time per iteration: {:?}", duration / ITERATIONS);
// }

// fn main() {
// 	benchmark_struct();
// 	benchmark_ref();
// }

use std::borrow::Cow;
use std::ptr::NonNull;
use std::rc::Rc;
// use std::alloc::{ self, Layout };

pub struct FaArray<T> {
	ptr: NonNull<T>,
	len: usize,
}

struct VecContainer<'a> {
	vec: Cow<'a, Vec<i32>>,
}

struct BoxContainer {
	boxe: Box<Vec<i32>>,
}

struct RcContainer {
	rc: Rc<Vec<i32>>,
}

struct VecTuple(Vec<i32>);

const ITERATIONS: u32 = 10;

fn log_container(container: &VecContainer) {
	println!("container: {:?}", container.vec);
}

fn log_tuple(tuple: &VecTuple) {
	println!("tuple: {:?}", tuple.0);
}

fn log_box_container(b: &BoxContainer) {
	println!("box: {:?}", b.boxe);
}

fn log_box(b: &Box<Vec<i32>>) {
	println!("box: {:?}", b);
}

fn log_container_vec<'scope>(vec: &'scope Vec<i32>) {
	println!("vec: {:?}", vec);
}

fn log_rc_container(container: &RcContainer) {
	println!("rc: {:?}", container.rc);
}

fn main() {
	let mut vec1 = VecContainer {
		vec: Cow::Owned(vec![1, 2, 3]),
	};

	let vec2: &VecContainer = &vec1; // Immutable borrow of vec1
	let vec3: &VecContainer = &vec1; // Another immutable borrow of vec1

	let tuple1 = VecTuple(vec![1, 2, 3]);
	let tuple2 = VecTuple(vec![1, 2, 3]);
	let tuple3 = VecTuple(vec![1, 2, 3]);

	let mut rc1 = RcContainer {
		rc: Rc::new(vec![7, 8]),
	};

	let rc2 = &rc1;

	let mut box1 = BoxContainer {
		boxe: Box::new(vec![4, 5]),
	};

	// push an element to the box
	box1.boxe.push(4);
	let box2 = &box1;
	let box3 = &box2;

	for _ in 0..ITERATIONS {
		log_container(&vec1);
		log_container(&(VecContainer { vec: Cow::Borrowed(&vec1.vec) }));
		// log_container_vec(&vec1.vec);

		// log_tuple(&tuple1);

		// log_box_container(&box1);
		// log_box_container(&box2);

		log_rc_container(&rc1);
		log_rc_container(
			&(RcContainer {
				rc: rc1.rc.clone(),
			})
		);

		// log_box_container(
		// 	&(BoxContainer {
		// 		boxe: box2.boxe.clone(),
		// 	})
		// );
		// log_tuple(&VecTuple(vec1.vec));
	}

	// println!("vec1: {:?}", vec1.vec);
	// println!("vec2: {:?}", vec2.vec);
	// println!("vec3: {:?}", vec3.vec);
}
