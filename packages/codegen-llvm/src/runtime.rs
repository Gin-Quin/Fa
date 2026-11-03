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
//! | `fa_alloc(size, align)` | `fa_heap_alloc` (size classes, tier 1; `malloc` under leak checks) | `fa_heap_alloc` | `fa_heap_alloc` |
//! | `fa_free(bytes, size, align)` | `fa_heap_free` (`free`) | `fa_heap_free` | `fa_heap_free` |
//! | `fa_pages(count)` (64 KiB pages for `fa_heap_alloc`) | `aligned_alloc` | `mmap` | `memory.grow` |
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
use inkwell::values::{FunctionValue, PointerValue};
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
	// the event loop of an asynchronous program on wasm (`lower/tasks.rs`)
	"fa_run",
	"fa_wake",
	"__multi3",
	"__udivti3",
	"__umodti3",
	"__divti3",
	"__modti3",
	"pow",
	"round",
];

const WASM_PAGE_SIZE: u64 = 65536;

/// The startup code and system calls of the freestanding Linux x86-64
/// target, as module-level assembly. `_start` is entered with a 16-byte
/// aligned stack holding `argc` then `argv`, so `call main(argc, argv)`
/// gives `main` the alignment of the ABI.
/// `memcpy`, `memmove` and `memset` are what LLVM calls for large or variable copies.
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
	.section .text.fa_sys_clock_gettime,"ax",@progbits
	.globl fa_sys_clock_gettime
fa_sys_clock_gettime:
	mov $228, %eax
	syscall
	ret
	.section .text.fa_sys_nanosleep,"ax",@progbits
	.globl fa_sys_nanosleep
fa_sys_nanosleep:
	mov $35, %eax
	syscall
	ret
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
	.section .text.memmove,"ax",@progbits
	.globl memmove
memmove:
	mov %rdi, %rax
	mov %rdx, %rcx
	cmp %rsi, %rdi
	jbe 1f
	lea -1(%rsi,%rdx), %rsi
	lea -1(%rdi,%rdx), %rdi
	std
	rep movsb
	cld
	ret
1:
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
	/// `fa_alloc` and `fa_free` count the live blocks (`Options::leak_check`).
	leak_check: bool,
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
			leak_check: false,
		}
	}

	/// Counts the live blocks in `fa_alloc` and `fa_free` (`fa_leak_check`).
	pub fn with_leak_check(mut self, enabled: bool) -> Self {
		self.leak_check = enabled;
		self
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
		// no threads without the C library: the state of each thread is global
		if self.target == Target::NativeFreestanding {
			for global in self.module.get_globals() {
				if global.is_thread_local() {
					global.set_thread_local(false);
				}
			}
		}
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
		self.pages()?;
		if self.module.get_function("fa_clock").is_some() {
			self.clock()?;
			self.nanosleep()?;
		}
		// with the C library, the compiler's own support library provides
		// the 128-bit builtins: the runtime's copies are left out
		if self.target == Target::Native {
			for name in ["__multi3", "__udivti3", "__umodti3", "__divti3", "__modti3"] {
				if let Some(builtin) = self.module.get_function(name) {
					for block in builtin.get_basic_blocks() {
						// SAFETY: the blocks belong to this function only
						unsafe { block.delete() }
							.map_err(|()| Error::Llvm(format!("can't drop `{name}`")))?;
					}
					builtin.set_linkage(Linkage::External);
				}
			}
		}
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
	/// allocation failure stops the program. The alignment is at most 16, the
	/// largest of a Fa type, which `malloc` and `fa_heap_alloc` give.
	pub fn alloc(&self) -> Result<FunctionValue<'ctx>, Error> {
		let ptr = self.context.ptr_type(AddressSpace::default());
		let r#type = ptr.fn_type(&[self.usize.into(), self.usize.into()], false);
		self.define("fa_alloc", r#type, &[], |builder, function| {
			// Each successful allocation is a fresh, non-overlapping block;
			// reused free-list blocks have no live owner. This is a return
			// property, not an assumption that user arguments cannot alias.
			for name in ["noalias", "nonnull"] {
				let attribute = self
					.context
					.create_enum_attribute(Attribute::get_named_enum_kind_id(name), 0);
				function.add_attribute(AttributeLoc::Return, attribute);
			}
			let size = function.get_nth_param(0).unwrap().into_int_value();
			let align = function.get_nth_param(1).unwrap().into_int_value();
			let failed = self.context.append_basic_block(function, "failed");
			let (out_of_memory, length) = self.bytes("fa_out_of_memory", b"stop: out of memory\n");
			let length = self.usize.const_int(length, false);
			let allocate = match self.target {
				Target::Native if self.leak_check => {
					self.declare("malloc", ptr.fn_type(&[self.usize.into()], false))
				}
				Target::Native | Target::NativeFreestanding | Target::Wasm => self
					.module
					.get_function("fa_heap_alloc")
					.ok_or_else(|| Error::Llvm("the runtime has no `fa_heap_alloc`".into()))?,
			};
			let succeeded = self.context.append_basic_block(function, "succeeded");
			let arguments: &[inkwell::values::BasicMetadataValueEnum<'ctx>] =
				if self.target == Target::Native && self.leak_check {
					&[size.into()]
				} else {
					&[size.into(), align.into()]
				};
			let bytes = builder
				.build_call(allocate, arguments, "bytes")?
				.try_as_basic_value()
				.basic()
				.unwrap()
				.into_pointer_value();
			let null = builder.build_is_null(bytes, "")?;
			builder.build_conditional_branch(null, failed, succeeded)?;
			builder.position_at_end(succeeded);
			self.count_block(builder, 1)?;
			builder.build_return(Some(&bytes))?;
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

	/// Adds `delta` to `fa_live_blocks` (tier 1) when counting blocks.
	fn count_block(&self, builder: &Builder<'ctx>, delta: i64) -> Result<(), Error> {
		if !self.leak_check {
			return Ok(());
		}
		let Some(counter) = self.module.get_global("fa_live_blocks") else {
			return Err(Error::Llvm("the runtime has no `fa_live_blocks`".into()));
		};
		let pointer = counter.as_pointer_value();
		// threads allocate and free at the same time (`for parallel`)
		builder.build_atomicrmw(
			inkwell::AtomicRMWBinOp::Add,
			pointer,
			self.usize.const_int(delta as u64, true),
			inkwell::AtomicOrdering::Monotonic,
		)?;
		Ok(())
	}

	/// `fa_free(ptr bytes, usize size, usize align)`: sized deallocation.
	pub fn free(&self) -> Result<FunctionValue<'ctx>, Error> {
		let ptr = self.context.ptr_type(AddressSpace::default());
		let r#type = self
			.context
			.void_type()
			.fn_type(&[ptr.into(), self.usize.into(), self.usize.into()], false);
		self.define("fa_free", r#type, &[], |builder, function| {
			let release = match self.target {
				Target::Native if self.leak_check => self.declare(
					"free",
					self.context.void_type().fn_type(&[ptr.into()], false),
				),
				Target::Native | Target::NativeFreestanding | Target::Wasm => self
					.module
					.get_function("fa_heap_free")
					.ok_or_else(|| Error::Llvm("the runtime has no `fa_heap_free`".into()))?,
			};
			let bytes = function.get_nth_param(0).unwrap();
			if self.leak_check && self.target == Target::Native {
				// a block read after it is freed reads 0xAB bytes (the tests)
				let usable = self.declare(
					"malloc_usable_size",
					self.usize.fn_type(&[ptr.into()], false),
				);
				let size = builder
					.build_call(usable, &[bytes.into()], "")?
					.try_as_basic_value()
					.basic()
					.unwrap()
					.into_int_value();
				let poison = self.context.i8_type().const_int(0xAB, false);
				builder
					.build_memset(bytes.into_pointer_value(), 1, poison, size)
					.map_err(|error| Error::Llvm(error.to_string()))?;
			}
			builder.build_call(release, &[bytes.into()], "")?;
			self.count_block(builder, -1)?;
			builder.build_return(None)?;
			Ok(())
		})
	}

	/// `fa_pages(usize count) -> ptr`: `count` new pages of 64 KiB, aligned
	/// on 64 KiB, for the allocator of the tier 1 runtime; null when the
	/// system has no more memory.
	pub fn pages(&self) -> Result<FunctionValue<'ctx>, Error> {
		let ptr = self.context.ptr_type(AddressSpace::default());
		let usize = self.usize;
		let r#type = ptr.fn_type(&[usize.into()], false);
		self.define("fa_pages", r#type, &[], |builder, function| {
			let count = function.get_nth_param(0).unwrap().into_int_value();
			let page_size = usize.const_int(WASM_PAGE_SIZE, false);
			match self.target {
				Target::Wasm => {
					let grow = self.intrinsic("llvm.wasm.memory.grow", &[usize.into()])?;
					let i32 = self.context.i32_type();
					let previous = builder
						.build_call(grow, &[i32.const_zero().into(), count.into()], "")?
						.try_as_basic_value()
						.basic()
						.unwrap()
						.into_int_value();
					let failed = builder.build_int_compare(
						IntPredicate::EQ,
						previous,
						usize.const_all_ones(),
						"",
					)?;
					let address = builder.build_int_mul(previous, page_size, "")?;
					let address = builder.build_int_to_ptr(address, ptr, "")?;
					let result = builder.build_select(failed, ptr.const_null(), address, "")?;
					builder.build_return(Some(&result))?;
				}
				Target::NativeFreestanding => {
					// one page more than asked, to align the start on 64 KiB
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
					let pages = builder.build_int_add(count, usize.const_int(1, false), "")?;
					let length = builder.build_int_mul(pages, page_size, "")?;
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
					let failed = builder.build_int_compare(
						IntPredicate::UGT,
						chunk,
						usize.const_int(-4096i64 as u64, true),
						"",
					)?;
					let mask = usize.const_int(WASM_PAGE_SIZE - 1, false);
					let aligned = builder.build_int_add(chunk, mask, "")?;
					let aligned = builder.build_and(aligned, builder.build_not(mask, "")?, "")?;
					let aligned = builder.build_int_to_ptr(aligned, ptr, "")?;
					let result = builder.build_select(failed, ptr.const_null(), aligned, "")?;
					builder.build_return(Some(&result))?;
				}
				Target::Native => {
					let aligned_alloc = self.declare(
						"aligned_alloc",
						ptr.fn_type(&[usize.into(), usize.into()], false),
					);
					let length = builder.build_int_mul(count, page_size, "")?;
					let result = builder
						.build_call(aligned_alloc, &[page_size.into(), length.into()], "")?
						.try_as_basic_value()
						.basic()
						.unwrap();
					builder.build_return(Some(&result))?;
				}
			}
			Ok(())
		})
	}

	/// `fa_clock() -> i64`: the monotonic clock in nanoseconds (on wasm,
	/// the host's `performance.now()`, imported as `fa.clock`).
	pub fn clock(&self) -> Result<FunctionValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let i32 = self.context.i32_type();
		let ptr = self.context.ptr_type(AddressSpace::default());
		let r#type = i64.fn_type(&[], false);
		self.define("fa_clock", r#type, &[], |builder, _| {
			if self.target == Target::Wasm {
				let f64 = self.context.f64_type();
				let host = self.import("clock", f64.fn_type(&[], false));
				let milliseconds = builder
					.build_call(host, &[], "")?
					.try_as_basic_value()
					.basic()
					.expect("a number")
					.into_float_value();
				let nanoseconds =
					builder.build_float_mul(milliseconds, f64.const_float(1e6), "")?;
				let nanoseconds = builder.build_float_to_signed_int(nanoseconds, i64, "")?;
				builder.build_return(Some(&nanoseconds))?;
				return Ok(());
			}
			let timespec = self.context.struct_type(&[i64.into(), i64.into()], false);
			let slot = builder.build_alloca(timespec, "time")?;
			let get_type = i32.fn_type(&[i32.into(), ptr.into()], false);
			let get = match self.target {
				Target::NativeFreestanding => self.system_call("clock_gettime", get_type),
				_ => self.declare("clock_gettime", get_type),
			};
			// CLOCK_MONOTONIC
			builder.build_call(get, &[i32.const_int(1, false).into(), slot.into()], "")?;
			let seconds = builder.build_struct_gep(timespec, slot, 0, "")?;
			let seconds = builder.build_load(i64, seconds, "")?.into_int_value();
			let nanoseconds = builder.build_struct_gep(timespec, slot, 1, "")?;
			let nanoseconds = builder.build_load(i64, nanoseconds, "")?.into_int_value();
			let total = builder.build_int_mul(seconds, i64.const_int(1_000_000_000, false), "")?;
			let total = builder.build_int_add(total, nanoseconds, "")?;
			builder.build_return(Some(&total))?;
			Ok(())
		})
	}

	/// `fa_nanosleep(i64 nanoseconds)`: a blocking sleep (nothing on wasm).
	pub fn nanosleep(&self) -> Result<FunctionValue<'ctx>, Error> {
		let i64 = self.context.i64_type();
		let i32 = self.context.i32_type();
		let ptr = self.context.ptr_type(AddressSpace::default());
		let r#type = self.context.void_type().fn_type(&[i64.into()], false);
		self.define("fa_nanosleep", r#type, &[], |builder, function| {
			if self.target == Target::Wasm {
				builder.build_return(None)?;
				return Ok(());
			}
			let duration = function.get_nth_param(0).unwrap().into_int_value();
			let timespec = self.context.struct_type(&[i64.into(), i64.into()], false);
			let slot = builder.build_alloca(timespec, "time")?;
			let billion = i64.const_int(1_000_000_000, false);
			let seconds = builder.build_int_signed_div(duration, billion, "")?;
			let nanoseconds = builder.build_int_signed_rem(duration, billion, "")?;
			let field = builder.build_struct_gep(timespec, slot, 0, "")?;
			builder.build_store(field, seconds)?;
			let field = builder.build_struct_gep(timespec, slot, 1, "")?;
			builder.build_store(field, nanoseconds)?;
			let sleep_type = i32.fn_type(&[ptr.into(), ptr.into()], false);
			let sleep = match self.target {
				Target::NativeFreestanding => self.system_call("nanosleep", sleep_type),
				_ => self.declare("nanosleep", sleep_type),
			};
			builder.build_call(sleep, &[slot.into(), ptr.const_null().into()], "")?;
			builder.build_return(None)?;
			Ok(())
		})
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
