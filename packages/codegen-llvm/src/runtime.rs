//! The runtime of a program, emitted into its module.
//!
//! Two tiers (`design/llvm-backend-plan.md`):
//!
//! - **Tier 0, the platform layer**, built here as LLVM IR per target, on
//!   first use only, with `internal` linkage:
//!
//! | Runtime function | Native (libc) | Freestanding (Linux x86-64) | Wasm (imports of the `fa` module) |
//! | --- | --- | --- | --- |
//! | `fa_write(fd, bytes, length)` | `write` | the `write` system call | `fa.write(fd, bytes, length)` |
//! | `fa_exit(code)` | `exit` | `exit_group` | `fa.exit(code)` |
//! | `fa_alloc(size, align)` | `malloc` | a bump allocator over `mmap` chunks | a bump allocator over `memory.grow` |
//! | `fa_free(bytes, size, align)` | `free` | nothing yet (bump allocator) | nothing yet (bump allocator) |
//!
//! - **Tier 1**, `runtime/fa_runtime.c` (printing, strings, stops, errors),
//!   compiled by clang 18 to the IR checked in next to it (`generate.sh`),
//!   parsed and linked into the module by [`Runtime::link_tier1`]. Every
//!   definition is made `internal` once the program is built
//!   ([`Runtime::internalize`]), so the optimizer inlines what the program
//!   uses and removes the rest.
//!
//! The freestanding target has no libc at all: [`FREESTANDING_ASSEMBLY`]
//! provides `_start` and the system calls, each in its own section so that
//! the linker drops the unused ones.

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::{Linkage, Module};
use inkwell::types::{FunctionType, IntType};
use inkwell::values::{FunctionValue, GlobalValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

use crate::error::Error;
use crate::target::Target;

/// The exit status of a stopped program (values brief 2.7).
pub const STOP_EXIT_CODE: i32 = 101;

/// The tier 1 runtime for 64-bit targets.
const TIER1_64: &str = include_str!("../runtime/fa_runtime.64.ll");

/// The tier 1 runtime for wasm32.
const TIER1_WASM32: &str = include_str!("../runtime/fa_runtime.wasm32.ll");

/// The symbols kept external: the entry points, and the compiler builtins
/// that instruction selection calls by name.
const EXTERNAL: &[&str] = &[
	"main",
	"_start",
	// exported on wasm, for the loader
	"fa_flush",
	"__multi3",
	"__udivti3",
	"__umodti3",
	"__divti3",
	"__modti3",
	"pow",
];

const WASM_PAGE_SIZE: u64 = 65536;

/// The smallest `mmap` chunk of the freestanding bump allocator.
const CHUNK_SIZE: u64 = 1 << 20;

/// The startup code and system calls of the freestanding Linux x86-64
/// target, as module-level assembly. `_start` is entered with a 16-byte
/// aligned stack holding `argc` then `argv`, so `call main(argc, argv)`
/// gives `main` the alignment of the ABI.
/// `memcpy` and `memset` are what LLVM calls for large or variable copies.
pub const FREESTANDING_ASSEMBLY: &str = r#"
	.section .text._start,"ax",@progbits
	.globl _start
_start:
	xor %ebp, %ebp
	mov (%rsp), %edi
	lea 8(%rsp), %rsi
	call main
	mov %eax, %edi
	mov $231, %eax
	syscall
	.section .text.fa_sys_write,"ax",@progbits
	.globl fa_sys_write
fa_sys_write:
	mov $1, %eax
	syscall
	ret
	.section .text.fa_sys_exit,"ax",@progbits
	.globl fa_sys_exit
fa_sys_exit:
	mov $231, %eax
	syscall
	.section .text.fa_sys_mmap,"ax",@progbits
	.globl fa_sys_mmap
fa_sys_mmap:
	mov %rcx, %r10
	mov $9, %eax
	syscall
	ret
	.section .text.memcpy,"ax",@progbits
	.globl memcpy
memcpy:
	mov %rdi, %rax
	mov %rdx, %rcx
	rep movsb
	ret
	.section .text.memset,"ax",@progbits
	.globl memset
memset:
	mov %rdi, %r9
	mov %esi, %eax
	mov %rdx, %rcx
	rep stosb
	mov %r9, %rax
	ret
"#;

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
		if target == Target::NativeFreestanding {
			module.set_inline_assembly(FREESTANDING_ASSEMBLY);
		}
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

	/// Parses the tier 1 runtime of the target and links it into the module.
	/// Its functions keep their external linkage until [`Runtime::internalize`].
	pub fn link_tier1(&self) -> Result<(), Error> {
		let text = match self.target {
			Target::Wasm => TIER1_WASM32,
			Target::Native | Target::NativeFreestanding => TIER1_64,
		};
		let buffer = MemoryBuffer::create_from_memory_range_copy(text.as_bytes(), "fa_runtime");
		let runtime = self.context.create_module_from_ir(buffer)?;
		runtime.set_triple(&self.module.get_triple());
		runtime.set_data_layout(&self.module.get_data_layout());
		self.module.link_in_module(runtime)?;
		Ok(())
	}

	/// A function of the tier 1 runtime.
	pub fn function(&self, name: &str) -> Result<FunctionValue<'ctx>, Error> {
		self.module
			.get_function(name)
			.ok_or_else(|| Error::Llvm(format!("the runtime has no `{name}`")))
	}

	/// Defines the platform layer that the tier 1 runtime declares, then makes
	/// every definition `internal` but the entry points and the compiler
	/// builtins, so that the optimizer removes what the program doesn't use.
	pub fn internalize(&self) -> Result<(), Error> {
		self.write()?;
		self.free()?;
		self.exit()?;
		self.alloc()?;
		let mut function = self.module.get_first_function();
		while let Some(current) = function {
			let name = current.get_name().to_string_lossy().into_owned();
			if current.count_basic_blocks() > 0 && !EXTERNAL.contains(&name.as_str()) {
				current.set_linkage(Linkage::Internal);
			}
			function = current.get_next_function();
		}
		for global in self.module.get_globals() {
			if !global.is_declaration() && !global.get_name().to_bytes().starts_with(b"llvm.") {
				global.set_linkage(Linkage::Internal);
			}
		}
		Ok(())
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
			let libc_type = self
				.usize
				.fn_type(&[i32.into(), ptr.into(), self.usize.into()], false);
			let platform = match self.target {
				Target::Native => self.declare("write", libc_type),
				Target::NativeFreestanding => self.system_call("write", libc_type),
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
				Target::NativeFreestanding => self.system_call("exit", r#type),
				Target::Wasm => self.import("exit", r#type),
			};
			self.add_attributes(platform, &["noreturn", "nounwind"]);
			let code = function.get_nth_param(0).unwrap();
			builder.build_call(platform, &[code.into()], "")?;
			builder.build_unreachable()?;
			Ok(())
		})
	}

	/// `fa_alloc(usize size, usize align) -> ptr`. Never returns null: an
	/// allocation failure stops the program.
	pub fn alloc(&self) -> Result<FunctionValue<'ctx>, Error> {
		let ptr = self.context.ptr_type(AddressSpace::default());
		let r#type = ptr.fn_type(&[self.usize.into(), self.usize.into()], false);
		self.define("fa_alloc", r#type, &[], |builder, function| {
			let size = function.get_nth_param(0).unwrap().into_int_value();
			let align = function.get_nth_param(1).unwrap().into_int_value();
			let failed = self.context.append_basic_block(function, "failed");
			let (out_of_memory, length) = self.bytes("fa_out_of_memory", b"stop: out of memory\n");
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
				Target::NativeFreestanding => {
					self.build_chunk_alloc(builder, function, size, align, failed)?
				}
				Target::Wasm => self.build_bump_alloc(builder, function, size, align, failed)?,
			}
			builder.position_at_end(failed);
			let stderr = self.context.i32_type().const_int(2, false);
			let code = self
				.context
				.i32_type()
				.const_int(STOP_EXIT_CODE as u64, false);
			builder.build_call(
				self.write()?,
				&[stderr.into(), out_of_memory.into(), length.into()],
				"",
			)?;
			builder.build_call(self.exit()?, &[code.into()], "")?;
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

	/// The freestanding allocator of the prototype: a bump pointer in
	/// chunks of at least [`CHUNK_SIZE`] bytes from `mmap`.
	fn build_chunk_alloc(
		&self,
		builder: &Builder<'ctx>,
		function: FunctionValue<'ctx>,
		size: inkwell::values::IntValue<'ctx>,
		align: inkwell::values::IntValue<'ctx>,
		failed: inkwell::basic_block::BasicBlock<'ctx>,
	) -> Result<(), Error> {
		let ptr = self.context.ptr_type(AddressSpace::default());
		let usize = self.usize;
		let top = self.global("fa_heap_top", usize, Some(usize.const_zero()));
		let limit = self.global("fa_heap_limit", usize, Some(usize.const_zero()));
		let mask = builder.build_int_sub(align, usize.const_int(1, false), "mask")?;
		let not_mask = builder.build_not(mask, "")?;
		let align_up = |value| -> Result<_, Error> {
			let value = builder.build_int_add(value, mask, "")?;
			Ok(builder.build_and(value, not_mask, "")?)
		};

		let current = builder
			.build_load(usize, top.as_pointer_value(), "top")?
			.into_int_value();
		let end_of_chunk = builder
			.build_load(usize, limit.as_pointer_value(), "limit")?
			.into_int_value();
		let aligned = align_up(current)?;
		let end = builder.build_int_add(aligned, size, "end")?;
		let wrapped = builder.build_int_compare(IntPredicate::ULT, end, aligned, "")?;
		let beyond = builder.build_int_compare(IntPredicate::UGT, end, end_of_chunk, "")?;
		let unset = builder.build_int_compare(IntPredicate::EQ, current, usize.const_zero(), "")?;
		let refill = builder.build_or(builder.build_or(wrapped, beyond, "")?, unset, "")?;
		let fits = self.context.append_basic_block(function, "fits");
		let map = self.context.append_basic_block(function, "map");
		let mapped = self.context.append_basic_block(function, "mapped");
		builder.build_conditional_branch(refill, map, fits)?;

		builder.position_at_end(fits);
		builder.build_store(top.as_pointer_value(), end)?;
		builder.build_return(Some(&builder.build_int_to_ptr(aligned, ptr, "")?))?;

		builder.position_at_end(map);
		// A chunk of max(size + align, CHUNK_SIZE) bytes; a size so large that
		// this wraps fails in mmap.
		let needed = builder.build_int_add(size, align, "")?;
		let small = builder.build_int_compare(
			IntPredicate::ULT,
			needed,
			usize.const_int(CHUNK_SIZE, false),
			"",
		)?;
		let length = builder
			.build_select(small, usize.const_int(CHUNK_SIZE, false), needed, "length")?
			.into_int_value();
		let i32 = self.context.i32_type();
		let mmap = self.system_call(
			"mmap",
			usize.fn_type(
				&[
					usize.into(),
					usize.into(),
					i32.into(),
					i32.into(),
					i32.into(),
					usize.into(),
				],
				false,
			),
		);
		const PROT_READ_WRITE: u64 = 0x3;
		const MAP_PRIVATE_ANONYMOUS: u64 = 0x22;
		let chunk = builder
			.build_call(
				mmap,
				&[
					usize.const_zero().into(),
					length.into(),
					i32.const_int(PROT_READ_WRITE, false).into(),
					i32.const_int(MAP_PRIVATE_ANONYMOUS, false).into(),
					i32.const_all_ones().into(),
					usize.const_zero().into(),
				],
				"chunk",
			)?
			.try_as_basic_value()
			.basic()
			.unwrap()
			.into_int_value();
		// Linux returns -errno, in [-4095, -1], on failure.
		let error = builder.build_int_compare(
			IntPredicate::UGT,
			chunk,
			usize.const_int(-4096i64 as u64, true),
			"",
		)?;
		builder.build_conditional_branch(error, failed, mapped)?;

		builder.position_at_end(mapped);
		let aligned = align_up(chunk)?;
		let end = builder.build_int_add(aligned, size, "")?;
		builder.build_store(top.as_pointer_value(), end)?;
		let chunk_end = builder.build_int_add(chunk, length, "")?;
		builder.build_store(limit.as_pointer_value(), chunk_end)?;
		builder.build_return(Some(&builder.build_int_to_ptr(aligned, ptr, "")?))?;
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

	/// Declares a system call stub of [`FREESTANDING_ASSEMBLY`].
	fn system_call(&self, name: &str, r#type: FunctionType<'ctx>) -> FunctionValue<'ctx> {
		self.declare(&format!("fa_sys_{name}"), r#type)
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
		let function = match self.module.get_function(name) {
			// defined already
			Some(function) if function.count_basic_blocks() > 0 => return Ok(function),
			// declared by the tier 1 runtime
			Some(function) => {
				function.set_linkage(Linkage::Internal);
				function
			}
			None => self
				.module
				.add_function(name, r#type, Some(Linkage::Internal)),
		};
		self.add_attributes(function, &["nounwind"]);
		self.add_attributes(function, attributes);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		body(&builder, function)?;
		Ok(function)
	}
}
