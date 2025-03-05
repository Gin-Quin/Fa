/* -------------------------------------------------------------------------- */
/*                                Atomic traits                               */
/* -------------------------------------------------------------------------- */

trait x_i32 {
	fn x(&self) -> i32;
}

trait y_i32 {
	fn y(&self) -> i32;
}

trait z_i32 {
	fn z(&self) -> i32;
}

/* -------------------------------------------------------------------------- */
/*                                 Structures                                 */
/* -------------------------------------------------------------------------- */

/* -------------------------------- MyStruct -------------------------------- */

struct MyStruct {
	x: i32,
	y: i32,
}

impl x_i32 for MyStruct {
	fn x(&self) -> i32 {
		self.x
	}
}

impl y_i32 for MyStruct {
	fn y(&self) -> i32 {
		self.y
	}
}

/* ------------------------------ MyOtherStruct ----------------------------- */

struct MyOtherStruct {
	x: i32,
	z: i32,
}

impl x_i32 for MyOtherStruct {
	fn x(&self) -> i32 {
		self.x
	}
}

impl z_i32 for MyOtherStruct {
	fn z(&self) -> i32 {
		self.z
	}
}

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */

fn main() {
	let my_struct = MyStruct { x: 1, y: 2 };
	let my_other_struct = MyOtherStruct { x: 3, z: 4 };
	let vec_of_structs: Vec<MyStruct> = vec![
		MyStruct { x: 1, y: 2 },
		MyStruct { x: 3, y: 4 }
	];
	let vec_of_other_structs: Vec<MyOtherStruct> = vec![
		MyOtherStruct { x: 1, z: 2 },
		MyOtherStruct { x: 3, z: 4 }
	];

	call_x(&my_struct);
	println!("-");
	call_x(&my_other_struct);

	println!("--------");

	call_x_y(&my_struct);

	println!("--------");

	call_many_x(&vec_of_structs);
	println!("-");
	call_many_x(&vec_of_other_structs);

	println!("--------");

	benchmark_nothing();
	benchmark_atomic_traits();
	benchmark_raw_structs();
}

fn call_x<HasX: x_i32>(x: &HasX) {
	println!("x: {}", x.x());
}

fn call_x_y<HasXAndY: x_i32 + y_i32>(x: &HasXAndY) {
	println!("x: {}", x.x());
	println!("y: {}", x.y());
}

fn call_many_x<HasX: x_i32>(vec: &Vec<HasX>) {
	for x in vec {
		call_x(x);
	}
}

fn check_x_equality<HasX1: x_i32, HasX2: x_i32>(
	_x: *const HasX1,
	_y: *const HasX2
) -> bool {
	let x = unsafe { &*_x };
	let y = unsafe { &*_y };
	x.x() == y.x()
}

fn check_x_equality_raw(myStruct: &MyStruct, myOtherStruct: &MyOtherStruct) -> bool {
	myStruct.x == myOtherStruct.x
}

/* -------------------------------------------------------------------------- */
/*                             Benchamrk functions                            */
/* -------------------------------------------------------------------------- */

const ITERATIONS: usize = 1_000_000;

fn benchmark_nothing() {
	let start = std::time::Instant::now();

	for _ in 0..ITERATIONS {
		// check_x_equality(&my_struct, &my_other_struct);
	}

	let duration = start.elapsed();
	println!("Nothing: {:?}", duration);
}

fn benchmark_atomic_traits() {
	let my_struct = MyStruct { x: 1, y: 2 };
	let my_other_struct = MyOtherStruct { x: 3, z: 4 };

	let start = std::time::Instant::now();

	for _ in 0..ITERATIONS {
		check_x_equality(&my_struct, &my_other_struct);
	}

	let duration = start.elapsed();
	println!("Atomic traits: {:?}", duration);
}

fn benchmark_raw_structs() {
	let my_struct = MyStruct { x: 1, y: 2 };
	let my_other_struct = MyOtherStruct { x: 3, z: 4 };

	let start = std::time::Instant::now();

	for _ in 0..ITERATIONS {
		check_x_equality_raw(&my_struct, &my_other_struct);
	}

	let duration = start.elapsed();
	println!("Raw structs: {:?}", duration);
}
