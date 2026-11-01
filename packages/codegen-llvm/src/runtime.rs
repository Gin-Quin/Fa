//! The minimal runtime, emitted by the compiler as LLVM IR into the module
//! that uses it.
//!
//! Each function is built on first use only, with `internal` linkage, so the
//! optimizer inlines it or deletes it with the rest of the program, and a
//! program pays only for what it calls. The functions are the same on every
//! target, down to a thin platform layer:
//!
//! | Runtime function | Native (libc) | Wasm (imports of the `fa` module) |
//! | --- | --- | --- |
//! | `fa_write(fd, bytes, length)` | `write` | `fa.write(fd, bytes, length)` |
//! | `fa_exit(code)` | `exit` | `fa.exit(code)` |
//! | `fa_alloc(size, align)` | `malloc` | a bump allocator over `memory.grow` |
//! | `fa_free(bytes, size, align)` | `free` | nothing yet (bump allocator) |
//!
//! Above it: `fa_print_i64`, `fa_panic` (writes to stderr and exits with
//! status 101) and `fa_overflow` (the cold, `noreturn` target of every
//! overflow check).

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{FunctionType, IntType};
use inkwell::values::{FunctionValue, GlobalValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

use crate::error::Error;
use crate::target::Target;

/// The exit status of a program stopped by a panic (as Rust's).
pub const PANIC_EXIT_CODE: i32 = 101;

/// The message printed when a checked arithmetic operation overflows.
pub const OVERFLOW_MESSAGE: &str = "panic: integer overflow\n";

const WASM_PAGE_SIZE: u64 = 65536;

/// Emits the runtime functions into a module, on demand.
pub struct Runtime<'ctx, 'module> {
	context: &'ctx Context,
	module: &'module Module<'ctx>,
	target: Target,
	/// The pointer-sized integer of the target (`i64` or `i32` on wasm32).
	usize: IntType<'ctx>,
}

impl<'ctx, 'module> Runtime<'ctx, 'module> {
	pub fn new(
		context: &'ctx Context,
		module: &'module Module<'ctx>,
		target: Target,
		usize: IntType<'ctx>,
	) -> Self {
		Runtime {
			context,
			module,
			target,
			usize,
		}
	}

	pub fn usize(&self) -> IntType<'ctx> {
		self.usize
	}

	/// An enum attribute, by its LLVM name.
	pub fn attribute(&self, name: &str) -> Attribute {
		let kind = Attribute::get_named_enum_kind_id(name);
		assert!(kind != 0, "unknown LLVM attribute `{name}`");
		self.context.create_enum_attribute(kind, 0)
	}

	/// Adds function attributes by name.
	pub fn add_attributes(&self, function: FunctionValue<'ctx>, names: &[&str]) {
		for name in names {
			function.add_attribute(AttributeLoc::Function, self.attribute(name));
		}
	}

	/// A private, constant, unnamed global holding `bytes`, and its length.
	pub fn bytes(&self, name: &str, bytes: &[u8]) -> (PointerValue<'ctx>, u64) {
		let global = match self.module.get_global(name) {
			Some(global) => global,
			None => {
				let value = self.context.const_string(bytes, false);
				let global = self.module.add_global(value.get_type(), None, name);
				global.set_initializer(&value);
				global.set_constant(true);
				global.set_linkage(Linkage::Private);
				global.set_unnamed_addr(true);
				global.set_alignment(1);
				global
			}
		};
		(global.as_pointer_value(), bytes.len() as u64)
	}

	/// `fa_write(i32 fd, ptr bytes, usize length)`.
	pub fn write(&self) -> Result<FunctionValue<'ctx>, Error> {
		let i32 = self.context.i32_type();
		let ptr = self.context.ptr_type(AddressSpace::default());
		let r#type = self
			.context
			.void_type()
			.fn_type(&[i32.into(), ptr.into(), self.usize.into()], false);
		self.define("fa_write", r#type, &[], |builder, function| {
			let fd = function.get_nth_param(0).unwrap();
			let bytes = function.get_nth_param(1).unwrap();
			let length = function.get_nth_param(2).unwrap();
			let platform = match self.target {
				Target::Native => self.declare(
					"write",
					self.usize
						.fn_type(&[i32.into(), ptr.into(), self.usize.into()], false),
				),
				Target::Wasm => self.import("write", r#type),
			};
			builder.build_call(platform, &[fd.into(), bytes.into(), length.into()], "")?;
			builder.build_return(None)?;
			Ok(())
		})
	}

	/// `fa_exit(i32 code)`, which does not return.
	pub fn exit(&self) -> Result<FunctionValue<'ctx>, Error> {
		let i32 = self.context.i32_type();
		let r#type = self.context.void_type().fn_type(&[i32.into()], false);
		self.define("fa_exit", r#type, &["noreturn"], |builder, function| {
			let platform = match self.target {
				Target::Native => self.declare("exit", r#type),
				Target::Wasm => self.import("exit", r#type),
			};
			self.add_attributes(platform, &["noreturn", "nounwind"]);
			let code = function.get_nth_param(0).unwrap();
			builder.build_call(platform, &[code.into()], "")?;
			builder.build_unreachable()?;
			Ok(())
		})
	}

	/// `fa_panic(ptr message, usize length)`: prints the message on the
	/// standard error and stops the program with [`PANIC_EXIT_CODE`].
	pub fn panic(&self) -> Result<FunctionValue<'ctx>, Error> {
		let ptr = self.context.ptr_type(AddressSpace::default());
		let r#type = self
			.context
			.void_type()
			.fn_type(&[ptr.into(), self.usize.into()], false);
		let write = self.write()?;
		let exit = self.exit()?;
		self.define(
			"fa_panic",
			r#type,
			&["cold", "noreturn", "noinline"],
			|builder, function| {
				let message = function.get_nth_param(0).unwrap();
				let length = function.get_nth_param(1).unwrap();
				let stderr = self.context.i32_type().const_int(2, false);
				builder.build_call(write, &[stderr.into(), message.into(), length.into()], "")?;
				let code = self
					.context
					.i32_type()
					.const_int(PANIC_EXIT_CODE as u64, false);
				builder.build_call(exit, &[code.into()], "")?;
				builder.build_unreachable()?;
				Ok(())
			},
		)
	}

	/// `fa_overflow()`: the shared, cold target of every overflow check, so
	/// that a check costs one compare and one call instruction.
	pub fn overflow(&self) -> Result<FunctionValue<'ctx>, Error> {
		let r#type = self.context.void_type().fn_type(&[], false);
		let panic = self.panic()?;
		self.define(
			"fa_overflow",
			r#type,
			&["cold", "noreturn", "noinline"],
			|builder, _| {
				let (message, length) =
					self.bytes("fa_overflow_message", OVERFLOW_MESSAGE.as_bytes());
				let length = self.usize.const_int(length, false);
				builder.build_call(panic, &[message.into(), length.into()], "")?;
				builder.build_unreachable()?;
				Ok(())
			},
		)
	}

	/// `fa_print_i64(i64 value)`: prints the decimal value and a new line,
	/// with one write.
	pub fn print_i64(&self) -> Result<FunctionValue<'ctx>, Error> {
		let i8 = self.context.i8_type();
		let i64 = self.context.i64_type();
		let r#type = self.context.void_type().fn_type(&[i64.into()], false);
		let write = self.write()?;
		self.define("fa_print_i64", r#type, &[], |builder, function| {
			const SIZE: u64 = 24; // a sign, 19 digits and a new line fit
			let value = function.get_nth_param(0).unwrap().into_int_value();
			let entry = builder.get_insert_block().unwrap();
			let digits = self.context.append_basic_block(function, "digits");
			let sign = self.context.append_basic_block(function, "sign");
			let print = self.context.append_basic_block(function, "print");
			let after_digits = self.context.append_basic_block(function, "after_digits");

			let buffer_type = i8.array_type(SIZE as u32);
			let buffer = builder.build_alloca(buffer_type, "buffer")?;
			let index = |builder: &Builder<'ctx>, position| unsafe {
				builder.build_in_bounds_gep(i8, buffer, &[position], "")
			};
			let last = i64.const_int(SIZE - 1, false);
			builder.build_store(index(builder, last)?, i8.const_int(b'\n' as u64, false))?;
			let negative = builder.build_int_compare(
				IntPredicate::SLT,
				value,
				i64.const_zero(),
				"negative",
			)?;
			// `0 - Int64.MIN` wraps to itself, which is the right magnitude unsigned.
			let negated = builder.build_int_sub(i64.const_zero(), value, "negated")?;
			let magnitude = builder.build_select(negative, negated, value, "magnitude")?;
			builder.build_unconditional_branch(digits)?;

			builder.position_at_end(digits);
			let position = builder.build_phi(i64, "position")?;
			let rest = builder.build_phi(i64, "rest")?;
			let ten = i64.const_int(10, false);
			let quotient =
				builder.build_int_unsigned_div(rest.as_basic_value().into_int_value(), ten, "")?;
			let product = builder.build_int_mul(quotient, ten, "")?;
			let digit =
				builder.build_int_sub(rest.as_basic_value().into_int_value(), product, "")?;
			let digit = builder.build_int_truncate(digit, i8, "")?;
			let digit = builder.build_int_add(digit, i8.const_int(b'0' as u64, false), "")?;
			let next = builder.build_int_sub(
				position.as_basic_value().into_int_value(),
				i64.const_int(1, false),
				"",
			)?;
			builder.build_store(index(builder, next)?, digit)?;
			let done =
				builder.build_int_compare(IntPredicate::EQ, quotient, i64.const_zero(), "")?;
			builder.build_conditional_branch(done, after_digits, digits)?;
			position.add_incoming(&[(&last, entry), (&next, digits)]);
			rest.add_incoming(&[(&magnitude, entry), (&quotient, digits)]);

			builder.position_at_end(after_digits);
			builder.build_conditional_branch(negative, sign, print)?;

			builder.position_at_end(sign);
			let signed = builder.build_int_sub(next, i64.const_int(1, false), "")?;
			builder.build_store(index(builder, signed)?, i8.const_int(b'-' as u64, false))?;
			builder.build_unconditional_branch(print)?;

			builder.position_at_end(print);
			let start = builder.build_phi(i64, "start")?;
			start.add_incoming(&[(&next, after_digits), (&signed, sign)]);
			let start = start.as_basic_value().into_int_value();
			let length = builder.build_int_sub(i64.const_int(SIZE, false), start, "")?;
			let length = builder.build_int_truncate_or_bit_cast(length, self.usize, "")?;
			let stdout = self.context.i32_type().const_int(1, false);
			let bytes = index(builder, start)?;
			builder.build_call(write, &[stdout.into(), bytes.into(), length.into()], "")?;
			builder.build_return(None)?;
			Ok(())
		})
	}

	/// `fa_alloc(usize size, usize align) -> ptr`. Never returns null: an
	/// allocation failure stops the program.
	pub fn alloc(&self) -> Result<FunctionValue<'ctx>, Error> {
		let ptr = self.context.ptr_type(AddressSpace::default());
		let r#type = ptr.fn_type(&[self.usize.into(), self.usize.into()], false);
		let panic = self.panic()?;
		self.define("fa_alloc", r#type, &[], |builder, function| {
			let size = function.get_nth_param(0).unwrap().into_int_value();
			let align = function.get_nth_param(1).unwrap().into_int_value();
			let failed = self.context.append_basic_block(function, "failed");
			let (out_of_memory, length) = self.bytes("fa_out_of_memory", b"panic: out of memory\n");
			let length = self.usize.const_int(length, false);
			match self.target {
				Target::Native => {
					// malloc aligns to 16 bytes, the largest alignment of a Fa
					// type in this prototype (an `aligned_alloc` path comes later).
					let _ = align;
					let malloc = self.declare("malloc", ptr.fn_type(&[self.usize.into()], false));
					let succeeded = self.context.append_basic_block(function, "succeeded");
					let bytes = builder
						.build_call(malloc, &[size.into()], "bytes")?
						.try_as_basic_value()
						.basic()
						.unwrap()
						.into_pointer_value();
					let null = builder.build_is_null(bytes, "")?;
					builder.build_conditional_branch(null, failed, succeeded)?;
					builder.position_at_end(succeeded);
					builder.build_return(Some(&bytes))?;
				}
				Target::Wasm => self.build_bump_alloc(builder, function, size, align, failed)?,
			}
			builder.position_at_end(failed);
			builder.build_call(panic, &[out_of_memory.into(), length.into()], "")?;
			builder.build_unreachable()?;
			Ok(())
		})
	}

	/// `fa_free(ptr bytes, usize size, usize align)`: sized deallocation.
	pub fn free(&self) -> Result<FunctionValue<'ctx>, Error> {
		let ptr = self.context.ptr_type(AddressSpace::default());
		let r#type = self
			.context
			.void_type()
			.fn_type(&[ptr.into(), self.usize.into(), self.usize.into()], false);
		self.define("fa_free", r#type, &[], |builder, function| {
			if self.target == Target::Native {
				let free = self.declare(
					"free",
					self.context.void_type().fn_type(&[ptr.into()], false),
				);
				let bytes = function.get_nth_param(0).unwrap();
				builder.build_call(free, &[bytes.into()], "")?;
			}
			builder.build_return(None)?;
			Ok(())
		})
	}

	/// The wasm allocator of the prototype: a bump pointer starting at
	/// `__heap_base` (defined by `wasm-ld`), growing the memory on demand.
	fn build_bump_alloc(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		size: inkwell::values::IntValue<'ctx>,
		align: inkwell::values::IntValue<'ctx>,
		failed: inkwell::basic_block::BasicBlock<'ctx>,
	) -> Result<(), Error> {
		let ptr = self.context.ptr_type(AddressSpace::default());
		let i32 = self.context.i32_type();
		let usize = self.usize;
		let top = self.global("fa_heap_top", usize, Some(usize.const_zero()));
		let heap_base = self.global("__heap_base", self.context.i8_type(), None);

		let current = builder
			.build_load(usize, top.as_pointer_value(), "top")?
			.into_int_value();
		let base = builder.build_ptr_to_int(heap_base.as_pointer_value(), usize, "base")?;
		let unset = builder.build_int_compare(IntPredicate::EQ, current, usize.const_zero(), "")?;
		let current = builder
			.build_select(unset, base, current, "current")?
			.into_int_value();
		let mask = builder.build_int_sub(align, usize.const_int(1, false), "")?;
		let aligned = builder.build_int_add(current, mask, "")?;
		let aligned = builder.build_and(aligned, builder.build_not(mask, "")?, "aligned")?;
		let end = builder.build_int_add(aligned, size, "end")?;
		let wrapped = builder.build_int_compare(IntPredicate::ULT, end, aligned, "")?;

		let memory_size = self.intrinsic("llvm.wasm.memory.size", &[usize.into()])?;
		let memory_grow = self.intrinsic("llvm.wasm.memory.grow", &[usize.into()])?;
		let pages = builder
			.build_call(memory_size, &[i32.const_zero().into()], "pages")?
			.try_as_basic_value()
			.basic()
			.unwrap()
			.into_int_value();
		let page_size = usize.const_int(WASM_PAGE_SIZE, false);
		let limit = builder.build_int_mul(pages, page_size, "limit")?;
		let grow = self.context.append_basic_block(function, "grow");
		let check_wrap = self.context.append_basic_block(function, "check_wrap");
		let done = self.context.append_basic_block(function, "done");
		let beyond = builder.build_int_compare(IntPredicate::UGT, end, limit, "")?;
		builder.build_conditional_branch(beyond, check_wrap, done)?;

		builder.position_at_end(check_wrap);
		builder.build_conditional_branch(wrapped, failed, grow)?;

		builder.position_at_end(grow);
		let missing = builder.build_int_sub(end, limit, "")?;
		let missing =
			builder.build_int_add(missing, usize.const_int(WASM_PAGE_SIZE - 1, false), "")?;
		let missing = builder.build_int_unsigned_div(missing, page_size, "missing_pages")?;
		let previous = builder
			.build_call(memory_grow, &[i32.const_zero().into(), missing.into()], "")?
			.try_as_basic_value()
			.basic()
			.unwrap()
			.into_int_value();
		let grow_failed =
			builder.build_int_compare(IntPredicate::EQ, previous, usize.const_all_ones(), "")?;
		builder.build_conditional_branch(grow_failed, failed, done)?;

		builder.position_at_end(done);
		builder.build_store(top.as_pointer_value(), end)?;
		let bytes = builder.build_int_to_ptr(aligned, ptr, "bytes")?;
		builder.build_return(Some(&bytes))?;
		Ok(())
	}

	fn global(
		&self,
		name: &str,
		r#type: impl inkwell::types::BasicType<'ctx>,
		initializer: Option<inkwell::values::IntValue<'ctx>>,
	) -> GlobalValue<'ctx> {
		if let Some(global) = self.module.get_global(name) {
			return global;
		}
		let global = self.module.add_global(r#type, None, name);
		if let Some(initializer) = initializer {
			global.set_initializer(&initializer);
			global.set_linkage(Linkage::Internal);
		}
		global
	}

	fn intrinsic(
		&self,
		name: &str,
		types: &[inkwell::types::BasicTypeEnum<'ctx>],
	) -> Result<FunctionValue<'ctx>, Error> {
		inkwell::intrinsics::Intrinsic::find(name)
			.and_then(|intrinsic| intrinsic.get_declaration(self.module, types))
			.ok_or_else(|| Error::Llvm(format!("unknown intrinsic {name}")))
	}

	/// Declares an external function of the platform (libc on native).
	fn declare(&self, name: &str, r#type: FunctionType<'ctx>) -> FunctionValue<'ctx> {
		self.module.get_function(name).unwrap_or_else(|| {
			let function = self
				.module
				.add_function(name, r#type, Some(Linkage::External));
			self.add_attributes(function, &["nounwind"]);
			function
		})
	}

	/// Declares a function imported from the `fa` module of the wasm host.
	fn import(&self, name: &str, r#type: FunctionType<'ctx>) -> FunctionValue<'ctx> {
		let symbol = format!("fa_host_{name}");
		self.module.get_function(&symbol).unwrap_or_else(|| {
			let function = self
				.module
				.add_function(&symbol, r#type, Some(Linkage::External));
			function.add_attribute(
				AttributeLoc::Function,
				self.context
					.create_string_attribute("wasm-import-module", "fa"),
			);
			function.add_attribute(
				AttributeLoc::Function,
				self.context
					.create_string_attribute("wasm-import-name", name),
			);
			self.add_attributes(function, &["nounwind"]);
			function
		})
	}

	/// Defines an internal `nounwind` runtime function once, with `body`
	/// building its blocks from the entry block.
	fn define(
		&self,
		name: &str,
		r#type: FunctionType<'ctx>,
		attributes: &[&str],
		body: impl FnOnce(&Builder<'ctx>, FunctionValue<'ctx>) -> Result<(), Error>,
	) -> Result<FunctionValue<'ctx>, Error> {
		if let Some(function) = self.module.get_function(name) {
			return Ok(function);
		}
		let function = self
			.module
			.add_function(name, r#type, Some(Linkage::Internal));
		self.add_attributes(function, &["nounwind"]);
		self.add_attributes(function, attributes);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		body(&builder, function)?;
		Ok(function)
	}
}
