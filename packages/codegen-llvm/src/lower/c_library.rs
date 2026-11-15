//! A program built as a C library (`build.cLibrary`, foreign-code.md,
//! "Exposing a library to C"; `fa_analyzer::c_library` lists the API).
//!
//! - **Wrappers**: each function of the namespace the API file exports is
//!   called by an exported `ccc` function, `<prefix>_<snake_case>`, with
//!   the C types of its parameters and result (structs by the target's C
//!   ABI, like `@extern` calls: `foreign.rs`). A function that throws
//!   returns an `int`: 0 on success (its result stored through a last
//!   pointer parameter, when it isn't null), else the position of the
//!   error's member in its `Errors` type, from 1; the error's text goes to
//!   a thread-local buffer of [`MESSAGE_SIZE`] bytes (a longer text is cut
//!   at a character), which `<prefix>_last_error_message()` returns (`""`
//!   before the first error): nothing to free, on any thread. A wrapper
//!   flushes the output of `console` before returning, so that it keeps
//!   its order with C's, and releases the thread's text builder (a thread
//!   of C may end after any call).
//! - **Load and unload**: `fa.library.init` (in `llvm.global_ctors`:
//!   `.init_array` on ELF, `__mod_init_func` on Mach-O) initializes the
//!   module values in the program's order and runs the API file's top
//!   level; `fa.library.fini` (in `llvm.global_dtors`) drops the module
//!   values when a drop does more than free memory (`destroy`) or the
//!   leaks are counted, then reports the leaks and flushes. Programs
//!   compiled per unit get both from their entry object (`units.rs`).
//! - The library runs in synchronous mode: an `async` function is exported
//!   as its blocking version.

use fa_analyzer::c_library::{self, ApiFunction};
use fa_analyzer::ids::{NodeId, TypeId};
use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::module::{Linkage, Module};
use inkwell::values::{
	BasicMetadataValueEnum, BasicValueEnum, FunctionValue, GlobalValue, PointerValue,
};

use crate::error::Error;
use crate::lower::foreign::{CType, Passing};
use crate::lower::values::{Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering};
use crate::runtime::Runtime;
use crate::target::Target;

/// Names containing `$` cannot collide with a C API identifier.
pub(crate) const WASM_EXPORTS: &[&str] =
	&["$fa.initialize", "$fa.finalize", "$fa.allocate", "$fa.free"];

/// Native libraries have load/unload hooks. Wasm hosts call explicit hooks;
/// the generated loader does that automatically and exposes dispose().
pub(crate) fn library_hooks<'ctx>(
	module: &Module<'ctx>,
	runtime: &Runtime<'ctx, '_>,
	target: Target,
	init: FunctionValue<'ctx>,
	fini: FunctionValue<'ctx>,
) -> Result<Vec<String>, Error> {
	if target != Target::Wasm {
		add_structor(module, init, true);
		add_structor(module, fini, false);
		return Ok(Vec::new());
	}
	let context = module.get_context();
	let i8 = context.i8_type();
	let state = module.add_global(i8, None, "fa.library.state");
	state.set_initializer(&i8.const_zero());
	state.set_linkage(Linkage::Internal);
	let builder = context.create_builder();
	for (name, body, before, after) in
		[(WASM_EXPORTS[0], init, 0, 1), (WASM_EXPORTS[1], fini, 1, 2)]
	{
		let wrapper = module.add_function(
			name,
			context.void_type().fn_type(&[], false),
			Some(Linkage::External),
		);
		runtime.add_attributes(wrapper, &["nounwind"]);
		let entry = context.append_basic_block(wrapper, "entry");
		let run = context.append_basic_block(wrapper, "run");
		let end = context.append_basic_block(wrapper, "end");
		builder.position_at_end(entry);
		let current = builder
			.build_load(i8, state.as_pointer_value(), "")?
			.into_int_value();
		let needed = builder.build_int_compare(
			inkwell::IntPredicate::EQ,
			current,
			i8.const_int(before, false),
			"",
		)?;
		builder.build_conditional_branch(needed, run, end)?;
		builder.position_at_end(run);
		builder.build_store(state.as_pointer_value(), i8.const_int(after, false))?;
		builder.build_call(body, &[], "")?;
		builder.build_unconditional_branch(end)?;
		builder.position_at_end(end);
		builder.build_return(None)?;
	}
	let usize = context.i32_type();
	let pointer = context.ptr_type(AddressSpace::default());
	let alloc = module.add_function(
		WASM_EXPORTS[2],
		pointer.fn_type(&[usize.into(), usize.into()], false),
		Some(Linkage::External),
	);
	let free = module.add_function(
		WASM_EXPORTS[3],
		context
			.void_type()
			.fn_type(&[pointer.into(), usize.into(), usize.into()], false),
		Some(Linkage::External),
	);
	for (wrapper, callee) in [(alloc, "fa_alloc"), (free, "fa_free")] {
		runtime.add_attributes(wrapper, &["nounwind"]);
		builder.position_at_end(context.append_basic_block(wrapper, "entry"));
		let arguments: Vec<BasicMetadataValueEnum<'ctx>> =
			wrapper.get_params().into_iter().map(Into::into).collect();
		let result = builder.build_call(runtime.function(callee)?, &arguments, "")?;
		if let Some(value) = result.try_as_basic_value().basic() {
			builder.build_return(Some(&value))?;
		} else {
			builder.build_return(None)?;
		}
	}
	Ok(WASM_EXPORTS
		.iter()
		.map(|name| (*name).to_string())
		.collect())
}

/// The thread-local message of the last error, zero-terminated.
const ERROR_MESSAGE: &str = "fa.c_library.error";
/// The size of the message's buffer, its final zero byte included.
const MESSAGE_SIZE: u32 = 1024;

/// Adds a function to `llvm.global_ctors` (`constructors`) or
/// `llvm.global_dtors`: run when the library is loaded, or unloaded.
pub(crate) fn add_structor<'ctx>(
	module: &Module<'ctx>,
	function: FunctionValue<'ctx>,
	constructors: bool,
) {
	let context = module.get_context();
	let i32 = context.i32_type();
	let pointer = context.ptr_type(AddressSpace::default());
	let entry_type = context.struct_type(&[i32.into(), pointer.into(), pointer.into()], false);
	let entry = entry_type.const_named_struct(&[
		i32.const_int(65535, false).into(),
		function.as_global_value().as_pointer_value().into(),
		pointer.const_null().into(),
	]);
	let array = entry_type.const_array(&[entry]);
	let name = if constructors {
		"llvm.global_ctors"
	} else {
		"llvm.global_dtors"
	};
	let global = module.add_global(array.get_type(), None, name);
	global.set_linkage(Linkage::Appending);
	global.set_initializer(&array);
}

/// Makes the thread-local variables of a module of a C library (the
/// runtime's are `initial-exec`) use the general dynamic model, which a
/// shared library loaded with `dlopen` (Python's `ctypes`) needs: the
/// static model takes a block of the static TLS area, which a library
/// loaded after the program's start can't get.
pub(crate) fn use_dynamic_tls(module: &Module<'_>) {
	for global in module.get_globals() {
		if global.is_thread_local() {
			global.set_thread_local_mode(Some(inkwell::ThreadLocalMode::GeneralDynamicTLSModel));
		}
	}
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// Whether the file being lowered is the API file of the C library.
	pub(crate) fn is_c_api_file(&self) -> bool {
		self.c_library
			.as_ref()
			.is_some_and(|library| library.api_module() == self.session.module(self.file))
	}

	/// The thread-local buffer of the last error's message.
	fn error_message_global(&self) -> GlobalValue<'ctx> {
		if let Some(global) = self.module.get_global(ERROR_MESSAGE) {
			return global;
		}
		let ty = self.context.i8_type().array_type(MESSAGE_SIZE);
		let global = self.module.add_global(ty, None, ERROR_MESSAGE);
		global.set_initializer(&ty.const_zero());
		global.set_thread_local(self.target != Target::Wasm);
		global.set_linkage(Linkage::Internal);
		global
	}

	/// `llvm.threadlocal.address.p0`: the address of a thread-local global
	/// on the calling thread.
	fn thread_local_address(&self) -> FunctionValue<'ctx> {
		let name = "llvm.threadlocal.address.p0";
		self.module.get_function(name).unwrap_or_else(|| {
			let pointer = self.pointer_type();
			self.module
				.add_function(name, pointer.fn_type(&[pointer.into()], false), None)
		})
	}

	fn error_message_address(&self, builder: &Builder<'ctx>) -> Result<PointerValue<'ctx>, Error> {
		let pointer = self.error_message_global().as_pointer_value();
		if self.target == Target::Wasm {
			return Ok(pointer);
		}
		Ok(builder
			.build_call(self.thread_local_address(), &[pointer.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("an address")
			.into_pointer_value())
	}

	/// The wrappers of the API file's functions (the file being lowered),
	/// and `<prefix>_last_error_message`.
	pub(crate) fn lower_c_api(&mut self) -> Result<(), Error> {
		let Some(library) = self.c_library.clone() else {
			return Ok(());
		};
		let interface = &self.session.source_file(self.file).interface;
		let Some(functions) = c_library::api_functions(self.types(), interface, &library.prefix)
		else {
			return Err(Error::Unsupported {
				message: format!(
					"a C library whose API (`{}`) doesn't export a namespace",
					library.api
				),
				location: self.path.clone(),
			});
		};
		for function in &functions {
			self.c_wrapper(function)?;
		}
		self.last_error_message(&library.prefix)?;
		Ok(())
	}

	/// The end of a call from C: the output of `console` flushed (it keeps
	/// its order with C's), and the text builder of the thread released (a
	/// thread of C may end after any call).
	fn end_c_call(&self) -> Result<(), Error> {
		self.call_runtime("fa_flush", &[])?;
		self.call_runtime("fa_release_builder", &[])
	}

	/// `<prefix>_last_error_message()`.
	fn last_error_message(&mut self, prefix: &str) -> Result<(), Error> {
		let name = format!("{prefix}_last_error_message");
		let ty = self.pointer_type().fn_type(&[], false);
		let function = self.module.add_function(&name, ty, Some(Linkage::External));
		self.runtime.add_attributes(function, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		// the address of the calling thread's buffer
		let message = self.error_message_address(&builder)?;
		builder.build_return(Some(&message))?;
		self.c_exports.push(name);
		Ok(())
	}

	/// The exported wrapper of an API function.
	fn c_wrapper(&mut self, api: &ApiFunction) -> Result<(), Error> {
		let node = api.decl;
		let Some(callee) = self.functions.get(&node).cloned() else {
			return Err(self.unsupported(node.index(), "this function of the C library"));
		};
		if callee.lambda {
			return Err(self.unsupported(
				node.index(),
				"a function of the C library that captures values",
			));
		}
		let Some(sig) = self.signature(api.ty) else {
			return Err(self.unsupported(node.index(), "this function of the C library"));
		};
		let param_types: Vec<TypeId> = self
			.types()
			.params(sig.params)
			.iter()
			.map(|param| param.ty)
			.collect();
		let c_params = param_types
			.iter()
			.map(|&ty| self.c_type(ty, node))
			.collect::<Result<Vec<_>, _>>()?;
		let c_ret = if sig.ret == TypeId::NEVER {
			CType::Void
		} else {
			self.c_type(sig.ret, node)?
		};
		let errors = c_library::error_type(self.types(), sig.throws).map_err(|_| {
			self.unsupported(
				node.index(),
				"a function of the C library throwing these errors",
			)
		})?;
		// a function that throws: `int f(parameters…, T *result)`
		let (function_type, attributes, out) = match errors {
			Some(_) => {
				let (inner, attributes) = self.c_function_type(&c_params, CType::Void, node)?;
				let mut params = inner.get_param_types();
				let out = (c_ret != CType::Void).then_some(params.len() as u32);
				if out.is_some() {
					params.push(self.pointer_type().into());
				}
				(
					self.context.i32_type().fn_type(&params, false),
					attributes,
					out,
				)
			}
			None => {
				let (ty, attributes) = self.c_function_type(&c_params, c_ret, node)?;
				(ty, attributes, None)
			}
		};
		let wrapper = self
			.module
			.add_function(&api.symbol, function_type, Some(Linkage::External));
		self.add_c_attributes(
			wrapper,
			attributes,
			if errors.is_some() { CType::Void } else { c_ret },
		);
		self.runtime.add_attributes(wrapper, &["nounwind"]);
		self.begin_function(wrapper, Repr::Void, Some(node));
		// the C arguments, as the Fa function takes them
		let mut position = 0u32;
		let ret_passing = match errors {
			Some(_) => None,
			None => self.passing(c_ret, node, true)?,
		};
		let sret = match &ret_passing {
			Some(Passing::Memory(_)) => {
				position += 1;
				wrapper
					.get_nth_param(0)
					.map(|param| param.into_pointer_value())
			}
			_ => None,
		};
		let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
		for (index, &c) in c_params.iter().enumerate() {
			let raw: BasicValueEnum<'ctx> = match self.passing(c, node, false)? {
				Some(Passing::Direct(_)) => {
					let raw = wrapper.get_nth_param(position).expect("a parameter");
					position += 1;
					raw
				}
				Some(Passing::Coerced(registers)) => {
					let structure = self.c_llvm_type(c).expect("a struct");
					let coerced = self.context.struct_type(&registers, false);
					let slot = self.coercion_slot(structure, coerced)?;
					for (field, register) in registers.iter().enumerate() {
						let raw = wrapper.get_nth_param(position).expect("a register");
						position += 1;
						let pointer =
							self.builder
								.build_struct_gep(coerced, slot, field as u32, "")?;
						let _ = register;
						self.builder.build_store(pointer, raw)?;
					}
					self.builder.build_load(structure, slot, "")?
				}
				Some(Passing::Memory(_) | Passing::Reference) => {
					let structure = self.c_llvm_type(c).expect("a struct");
					let pointer = wrapper
						.get_nth_param(position)
						.expect("a pointer")
						.into_pointer_value();
					position += 1;
					self.builder.build_load(structure, pointer, "")?
				}
				None => return Err(self.unsupported(node.index(), "a `Void` parameter")),
			};
			let value = self.c_to_fa(raw, param_types[index], node)?;
			let (repr, has_default) = callee.params[index];
			let raw = value.value.expect("a value");
			if callee.by_pointer[index] {
				let slot = self.alloca(repr, "c_parameter")?;
				self.builder.build_store(slot.pointer, raw)?;
				arguments.push(slot.pointer.into());
			} else {
				arguments.push(raw.into());
			}
			if has_default {
				arguments.push(self.context.bool_type().const_int(1, false).into());
			}
		}
		// the storage the Fa function takes for its result: a slot of the
		// wrapper, no buffer (`return_destinations.rs`, `collection_storage.rs`)
		if callee.return_collection {
			arguments.insert(0, self.pointer_type().const_null().into());
			arguments.insert(1, self.usize.const_zero().into());
		}
		let return_slot = if callee.return_slot {
			let slot = self.alloca(callee.ret, "c_result")?;
			arguments.insert(0, slot.pointer.into());
			Some(slot)
		} else {
			None
		};
		let call = self.builder.build_call(callee.value, &arguments, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		let returned = call.try_as_basic_value().basic();
		// the result and the error
		let by_value = callee.ret.has_value() && return_slot.is_none();
		let (result, error) = match (errors.is_some(), by_value) {
			(false, _) => (returned, None),
			(true, true) => {
				let pair = returned.expect("a pair").into_struct_value();
				let value = self.builder.build_extract_value(pair, 0, "")?;
				let error = self.builder.build_extract_value(pair, 1, "")?;
				(Some(value), Some(error.into_pointer_value()))
			}
			(true, false) => (None, returned.map(|error| error.into_pointer_value())),
		};
		if let (Some(nominal), Some(error)) = (errors, error) {
			let failed = self.append_block("c_error");
			let succeeded = self.append_block("c_success");
			let is_error = self.builder.build_is_not_null(error, "")?;
			self.builder
				.build_conditional_branch(is_error, failed, succeeded)?;
			self.builder.position_at_end(failed);
			let status = self.c_error_status(nominal, error)?;
			self.end_c_call()?;
			self.builder.build_return(Some(&status))?;
			self.builder.position_at_end(succeeded);
		}
		// a result built in the slot is read once the call succeeded
		let result = match return_slot {
			Some(slot) => self.load(slot)?.value,
			None => result,
		};
		let i32 = self.context.i32_type();
		match (errors, result) {
			(Some(_), result) => {
				if let (Some(out), Some(raw)) = (out, result) {
					let pointer = wrapper
						.get_nth_param(out)
						.expect("the result's pointer")
						.into_pointer_value();
					let c_value = self.to_c(Val::new(callee.ret, raw), c_ret, node)?;
					let given = self.append_block("c_result");
					let after = self.append_block("c_returned");
					let present = self.builder.build_is_not_null(pointer, "")?;
					self.builder
						.build_conditional_branch(present, given, after)?;
					self.builder.position_at_end(given);
					self.builder.build_store(pointer, c_value)?;
					self.builder.build_unconditional_branch(after)?;
					self.builder.position_at_end(after);
				}
				self.end_c_call()?;
				self.builder.build_return(Some(&i32.const_zero()))?;
			}
			(None, Some(raw)) if c_ret != CType::Void => {
				let c_value = self.to_c(Val::new(callee.ret, raw), c_ret, node)?;
				self.end_c_call()?;
				match ret_passing {
					Some(Passing::Memory(_)) => {
						self.builder
							.build_store(sret.expect("the result's block"), c_value)?;
						self.builder.build_return(None)?;
					}
					Some(Passing::Coerced(registers)) => {
						let structure = self.c_llvm_type(c_ret).expect("a struct");
						let coerced = self.context.struct_type(&registers, false);
						let slot = self.coercion_slot(structure, coerced)?;
						self.builder.build_store(slot, c_value)?;
						let value = if registers.len() == 1 {
							self.builder.build_load(registers[0], slot, "")?
						} else {
							self.builder.build_load(coerced, slot, "")?
						};
						self.builder.build_return(Some(&value))?;
					}
					_ => {
						self.builder.build_return(Some(&c_value))?;
					}
				}
			}
			(None, _) => {
				if callee.ret == Repr::Never {
					self.builder.build_unreachable()?;
				} else {
					self.end_c_call()?;
					self.builder.build_return(None)?;
				}
			}
		}
		self.end_function()?;
		self.c_exports.push(api.symbol.clone());
		Ok(())
	}

	/// The status of an error (the position of its member, from 1; -1 for
	/// another), its text stored as the thread's last error message, and
	/// the error dropped.
	fn c_error_status(
		&mut self,
		nominal: fa_analyzer::ids::NominalId,
		error: inkwell::values::PointerValue<'ctx>,
	) -> Result<inkwell::values::IntValue<'ctx>, Error> {
		let i32 = self.context.i32_type();
		let i64 = self.context.i64_type();
		let count = self
			.types()
			.sym_list(self.types().nominal(nominal).members)
			.len();
		let id = self.builder.build_load(i64, error, "id")?.into_int_value();
		let mut status = i32.const_int(u64::MAX, true);
		for index in (0..count).rev() {
			let member = self.member_id(nominal, index as u16);
			let matches = self.builder.build_int_compare(
				inkwell::IntPredicate::EQ,
				id,
				i64.const_int(member, false),
				"",
			)?;
			status = self
				.builder
				.build_select(matches, i32.const_int(index as u64 + 1, false), status, "")?
				.into_int_value();
		}
		// the text: the error as `console.log` shows it
		self.call_runtime("fa_b_reset", &[])?;
		let nested = i32.const_int(1, false);
		self.call_runtime("fa_format_union", &[error.into(), nested.into()])?;
		let text = self.call_with_pair("fa_b_take", &[], Repr::Str)?;
		let (bytes, length) = self.string_parts(text)?;
		let length = self.builder.build_int_cast(length, self.usize, "")?;
		// at most the buffer's size but one, cut before a continuation byte
		let limit = self.usize.const_int(u64::from(MESSAGE_SIZE - 1), false);
		let fits = self
			.builder
			.build_int_compare(inkwell::IntPredicate::ULE, length, limit, "")?;
		let cut = self.append_block("c_message_cut");
		let copy = self.append_block("c_message_copy");
		let before = self.builder.get_insert_block().expect("a block");
		self.builder.build_conditional_branch(fits, copy, cut)?;
		self.builder.position_at_end(cut);
		let at = self.builder.build_phi(self.usize, "at")?;
		let position = at.as_basic_value().into_int_value();
		// SAFETY: `position` is below `length`
		let byte_pointer = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), bytes, &[position], "")?
		};
		let byte = self
			.builder
			.build_load(self.context.i8_type(), byte_pointer, "")?
			.into_int_value();
		let high =
			self.builder
				.build_and(byte, self.context.i8_type().const_int(0xc0, false), "")?;
		let continuation = self.builder.build_int_compare(
			inkwell::IntPredicate::EQ,
			high,
			self.context.i8_type().const_int(0x80, false),
			"",
		)?;
		let previous = self
			.builder
			.build_int_sub(position, self.usize.const_int(1, false), "")?;
		at.add_incoming(&[(&limit, before), (&previous, cut)]);
		self.builder
			.build_conditional_branch(continuation, cut, copy)?;
		self.builder.position_at_end(copy);
		let size = self.builder.build_phi(self.usize, "size")?;
		size.add_incoming(&[(&length, before), (&position, cut)]);
		let size = size.as_basic_value().into_int_value();
		let buffer = self.error_message_address(&self.builder)?;
		self.builder.build_memcpy(buffer, 1, bytes, 1, size)?;
		// SAFETY: `size` is below the buffer's size
		let end = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), buffer, &[size], "")?
		};
		self.builder
			.build_store(end, self.context.i8_type().const_zero())?;
		self.emit_drop(text)?;
		self.emit_drop(Val::new(Repr::Union, error))?;
		Ok(status)
	}

	/// The load and unload functions of a C library compiled as one module:
	/// `fa.library.init` initializes the module values and runs the API
	/// file's top level, `fa.library.fini` drops the values (when a drop
	/// does more than free memory, or the leaks are counted).
	pub(crate) fn lower_library_entry(&mut self) -> Result<(), Error> {
		let ty = self.context.void_type().fn_type(&[], false);
		let init = self
			.module
			.add_function("fa.library.init", ty, Some(Linkage::Internal));
		self.runtime.add_attributes(init, &["nounwind"]);
		self.begin_function(init, Repr::Void, None);
		self.initialize_modules()?;
		let root = NodeId::new(self.tree.root);
		let fa_analyzer::syntax::View::Module { statements } = self.view(root) else {
			return Err(self.unsupported(self.tree.root, "this file"));
		};
		let statements: Vec<NodeId> = statements
			.iter()
			.filter(|&statement| {
				!(self.module_file
					&& matches!(
						self.view(statement),
						fa_analyzer::syntax::View::Declaration { .. }
							| fa_analyzer::syntax::View::ExportValue { .. }
					))
			})
			.collect();
		let mark = self.mark();
		self.top_level_statements(&statements)?;
		self.end_statement(mark, None)?;
		let values = self.cleanups.take();
		if !self.terminated() {
			self.builder.build_return(None)?;
		}
		self.end_function()?;
		let fini = self
			.module
			.add_function("fa.library.fini", ty, Some(Linkage::Internal));
		self.runtime.add_attributes(fini, &["nounwind"]);
		self.begin_function(fini, Repr::Void, None);
		self.cleanups.replace(values);
		if self.leak_check {
			self.drop_to(0)?;
		} else {
			self.drop_at_exit()?;
		}
		if !self.terminated() {
			if self.leak_check {
				self.call_runtime("fa_leak_check", &[])?;
			}
			self.call_runtime("fa_flush", &[])?;
			self.builder.build_return(None)?;
		}
		self.end_function()?;
		let exports = library_hooks(self.module, &self.runtime, self.target, init, fini)?;
		self.c_exports.extend(exports);
		Ok(())
	}

	/// After the runtime is internalized: the C functions stay exported.
	pub(crate) fn keep_c_exports(&self) {
		for name in &self.c_exports {
			if let Some(function) = self.module.get_function(name) {
				function.set_linkage(Linkage::External);
			}
		}
	}
}
