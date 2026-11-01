use inkwell::IntPredicate;
use inkwell::attributes::AttributeLoc;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::intrinsics::Intrinsic;
use inkwell::module::{Linkage, Module};
use inkwell::values::{FunctionValue, IntValue};

use crate::error::Error;
use crate::runtime::Runtime;
use crate::target::Target;

/// LLVM's number for the `fastcc` calling convention.
const FAST_CALLING_CONVENTION: u32 = 8;

/// The programs of the proof of concept, built by hand as a lowering of a
/// typed Fa tree would build them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Program {
	/// ```fa
	/// function fibonacci = (n: Integer): Integer =>
	///     if n < 2 { n } else { fibonacci(n - 1) + fibonacci(n - 2) }
	/// print(fibonacci(30))
	/// ```
	/// `n - 1` and `n - 2` cannot overflow (`n >= 2`), the sum can.
	Fibonacci(i64),
	/// `print("Hello, world!")`.
	Hello,
	/// `print("Hello, " + "world!")` with a heap-allocated result.
	Concatenation,
	/// `print(Int64.MAX + 1)`, which stops with an overflow panic.
	Overflow,
}

impl Program {
	/// What the program prints on its standard output.
	pub fn expected_output(self) -> String {
		match self {
			Program::Fibonacci(n) => format!("{}\n", fibonacci(n)),
			Program::Hello | Program::Concatenation => "Hello, world!\n".to_string(),
			Program::Overflow => String::new(),
		}
	}

	pub fn name(self) -> String {
		match self {
			Program::Fibonacci(n) => format!("fibonacci_{n}"),
			Program::Hello => "hello".to_string(),
			Program::Concatenation => "concatenation".to_string(),
			Program::Overflow => "overflow".to_string(),
		}
	}

	/// Builds the module of the program, with its runtime.
	pub fn build<'ctx>(
		self,
		context: &'ctx Context,
		target: Target,
		usize: inkwell::types::IntType<'ctx>,
	) -> Result<Module<'ctx>, Error> {
		let module = context.create_module(&self.name());
		let runtime = Runtime::new(context, &module, target, usize);
		let builder = context.create_builder();
		let entry = entry_point(context, &module, &runtime, target, &builder);
		match self {
			Program::Fibonacci(n) => {
				let fibonacci = build_fibonacci(context, &module, &runtime)?;
				let argument = context.i64_type().const_int(n as u64, true);
				let call = builder.build_call(fibonacci, &[argument.into()], "result")?;
				call.set_call_convention(FAST_CALLING_CONVENTION);
				let result = call.try_as_basic_value().basic().unwrap();
				builder.build_call(runtime.print_i64()?, &[result.into()], "")?;
			}
			Program::Hello => {
				let (text, length) = runtime.bytes("hello", b"Hello, world!\n");
				let stdout = context.i32_type().const_int(1, false);
				let length = usize.const_int(length, false);
				builder.build_call(
					runtime.write()?,
					&[stdout.into(), text.into(), length.into()],
					"",
				)?;
			}
			Program::Concatenation => {
				let (left, left_length) = runtime.bytes("left", b"Hello, ");
				let (right, right_length) = runtime.bytes("right", b"world!\n");
				let size = usize.const_int(left_length + right_length, false);
				let one = usize.const_int(1, false);
				let call =
					builder.build_call(runtime.alloc()?, &[size.into(), one.into()], "text")?;
				let text = call
					.try_as_basic_value()
					.basic()
					.unwrap()
					.into_pointer_value();
				builder.build_memcpy(text, 1, left, 1, usize.const_int(left_length, false))?;
				let tail = unsafe {
					builder.build_in_bounds_gep(
						context.i8_type(),
						text,
						&[usize.const_int(left_length, false)],
						"tail",
					)?
				};
				builder.build_memcpy(tail, 1, right, 1, usize.const_int(right_length, false))?;
				let stdout = context.i32_type().const_int(1, false);
				builder.build_call(
					runtime.write()?,
					&[stdout.into(), text.into(), size.into()],
					"",
				)?;
				builder.build_call(runtime.free()?, &[text.into(), size.into(), one.into()], "")?;
			}
			Program::Overflow => {
				let maximum = context.i64_type().const_int(i64::MAX as u64, true);
				let one = context.i64_type().const_int(1, true);
				let sum = checked_add(context, &module, &runtime, &builder, entry, maximum, one)?;
				builder.build_call(runtime.print_i64()?, &[sum.into()], "")?;
			}
		}
		match target {
			Target::Native | Target::Freestanding => {
				builder.build_return(Some(&context.i32_type().const_zero()))?
			}
			Target::Wasm => builder.build_return(None)?,
		};
		Ok(module)
	}
}

/// The reference result, computed iteratively.
pub fn fibonacci(n: i64) -> i64 {
	let (mut current, mut next) = (0i64, 1i64);
	for _ in 0..n {
		(current, next) = (next, current + next);
	}
	current
}

/// `i32 main()` on native (called by `_start` when freestanding), the exported `void _start()` on wasm (the entry
/// point `wasm-ld` expects), with the builder positioned in its entry block.
fn entry_point<'ctx>(
	context: &'ctx Context,
	module: &Module<'ctx>,
	runtime: &Runtime<'ctx, '_>,
	target: Target,
	builder: &Builder<'ctx>,
) -> FunctionValue<'ctx> {
	let (name, r#type) = match target {
		Target::Native | Target::Freestanding => ("main", context.i32_type().fn_type(&[], false)),
		Target::Wasm => ("_start", context.void_type().fn_type(&[], false)),
	};
	let function = module.add_function(name, r#type, Some(Linkage::External));
	runtime.add_attributes(function, &["nounwind"]);
	builder.position_at_end(context.append_basic_block(function, "entry"));
	function
}

fn build_fibonacci<'ctx>(
	context: &'ctx Context,
	module: &Module<'ctx>,
	runtime: &Runtime<'ctx, '_>,
) -> Result<FunctionValue<'ctx>, Error> {
	let i64 = context.i64_type();
	let function = module.add_function(
		"fibonacci",
		i64.fn_type(&[i64.into()], false),
		Some(Linkage::Internal),
	);
	function.set_call_conventions(FAST_CALLING_CONVENTION);
	runtime.add_attributes(function, &["nounwind"]);
	// `n` is always initialized: `noundef`, as every Fa value.
	function.add_attribute(AttributeLoc::Param(0), runtime.attribute("noundef"));
	let builder = context.create_builder();
	let entry = context.append_basic_block(function, "entry");
	let small = context.append_basic_block(function, "small");
	let recurse = context.append_basic_block(function, "recurse");
	builder.position_at_end(entry);
	let n = function.get_nth_param(0).unwrap().into_int_value();
	let is_small = builder.build_int_compare(IntPredicate::SLT, n, i64.const_int(2, false), "")?;
	builder.build_conditional_branch(is_small, small, recurse)?;

	builder.position_at_end(small);
	builder.build_return(Some(&n))?;

	builder.position_at_end(recurse);
	// Proven by the range analysis: `n >= 2`, so these never overflow (`nsw`).
	let mut terms = Vec::new();
	for offset in [1, 2] {
		let argument = builder.build_int_nsw_sub(n, i64.const_int(offset, false), "")?;
		let call = builder.build_call(function, &[argument.into()], "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		terms.push(call.try_as_basic_value().basic().unwrap().into_int_value());
	}
	let sum = checked_add(
		context, module, runtime, &builder, function, terms[0], terms[1],
	)?;
	builder.build_return(Some(&sum))?;
	Ok(function)
}

/// `a + b` on `Int64` when the range analysis could not prove that it does
/// not overflow: `llvm.sadd.with.overflow`, and a branch weighted as never
/// taken to a block calling the cold `fa_overflow`.
fn checked_add<'ctx>(
	context: &'ctx Context,
	module: &Module<'ctx>,
	runtime: &Runtime<'ctx, '_>,
	builder: &Builder<'ctx>,
	function: FunctionValue<'ctx>,
	a: IntValue<'ctx>,
	b: IntValue<'ctx>,
) -> Result<IntValue<'ctx>, Error> {
	let i64 = context.i64_type();
	let add = Intrinsic::find("llvm.sadd.with.overflow")
		.and_then(|intrinsic| intrinsic.get_declaration(module, &[i64.into()]))
		.ok_or_else(|| Error::Llvm("llvm.sadd.with.overflow is missing".to_string()))?;
	let result = builder
		.build_call(add, &[a.into(), b.into()], "")?
		.try_as_basic_value()
		.basic()
		.unwrap()
		.into_struct_value();
	let sum = builder
		.build_extract_value(result, 0, "sum")?
		.into_int_value();
	let overflowed = builder
		.build_extract_value(result, 1, "overflowed")?
		.into_int_value();
	let overflow = context.append_basic_block(function, "overflow");
	let continuation = context.append_basic_block(function, "no_overflow");
	let branch = builder.build_conditional_branch(overflowed, overflow, continuation)?;
	let i32 = context.i32_type();
	let weights = context.metadata_node(&[
		context.metadata_string("branch_weights").into(),
		i32.const_int(1, false).into(),
		i32.const_int(1 << 20, false).into(),
	]);
	branch
		.set_metadata(weights, context.get_kind_id("prof"))
		.map_err(|error| Error::Llvm(error.to_string()))?;

	builder.position_at_end(overflow);
	builder.build_call(runtime.overflow()?, &[], "")?;
	builder.build_unreachable()?;

	builder.position_at_end(continuation);
	Ok(sum)
}
