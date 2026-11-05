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
//!   a thread-local block of `malloc`, which `<prefix>_last_error_message()`
//!   returns (`""` before the first error). A wrapper flushes the output
//!   of `console` before returning, so that it keeps its order with C's.
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
use inkwell::module::{Linkage, Module};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, GlobalValue};

use crate::error::Error;
use crate::lower::foreign::{CType, Passing};
use crate::lower::values::{Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Lowering};

/// The thread-local message of the last error (a block of `malloc`).
const ERROR_MESSAGE: &str = "fa.c_library.error";
/// Frees the calling thread's message (the unload).
pub(crate) const RELEASE: &str = "fa.c_library.release";

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

impl<'ctx> Lowering<'_, 'ctx> {
	/// Whether the file being lowered is the API file of the C library.
	pub(crate) fn is_c_api_file(&self) -> bool {
		self.c_library
			.as_ref()
			.is_some_and(|library| library.api_module() == self.session.module(self.file))
	}

	/// The thread-local message of the last error.
	fn error_message_global(&self) -> GlobalValue<'ctx> {
		if let Some(global) = self.module.get_global(ERROR_MESSAGE) {
			return global;
		}
		let global = self
			.module
			.add_global(self.pointer_type(), None, ERROR_MESSAGE);
		global.set_initializer(&self.pointer_type().const_null());
		global.set_thread_local(true);
		global.set_linkage(Linkage::Internal);
		global
	}

	/// A function of the C library (`malloc`, `free`), declared once.
	fn libc_function(&self, name: &str) -> FunctionValue<'ctx> {
		if let Some(function) = self.module.get_function(name) {
			return function;
		}
		let pointer = self.pointer_type();
		let ty = match name {
			"malloc" => pointer.fn_type(&[self.usize.into()], false),
			_ => self.context.void_type().fn_type(&[pointer.into()], false),
		};
		self.module.add_function(name, ty, Some(Linkage::External))
	}

	/// The wrappers of the API file's functions (the file being lowered),
	/// `<prefix>_last_error_message` and the release of the message.
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
		self.release_function()?;
		Ok(())
	}

	/// `<prefix>_last_error_message()`.
	fn last_error_message(&mut self, prefix: &str) -> Result<(), Error> {
		let name = format!("{prefix}_last_error_message");
		let ty = self.pointer_type().fn_type(&[], false);
		let function = self.module.add_function(&name, ty, Some(Linkage::External));
		self.runtime.add_attributes(function, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		let global = self.error_message_global();
		let message = builder
			.build_load(self.pointer_type(), global.as_pointer_value(), "")?
			.into_pointer_value();
		let empty = self.constant_bytes(&[0]);
		let missing = builder.build_is_null(message, "")?;
		let result = builder.build_select(missing, empty, message, "")?;
		builder.build_return(Some(&result))?;
		self.c_exports.push(name);
		Ok(())
	}

	/// `fa.c_library.release()`: frees the calling thread's message.
	fn release_function(&mut self) -> Result<(), Error> {
		let ty = self.context.void_type().fn_type(&[], false);
		let function = self
			.module
			.add_function(RELEASE, ty, Some(Linkage::External));
		self.runtime.add_attributes(function, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		let global = self.error_message_global().as_pointer_value();
		let message = builder.build_load(self.pointer_type(), global, "")?;
		builder.build_call(self.libc_function("free"), &[message.into()], "")?;
		builder.build_store(global, self.pointer_type().const_null())?;
		builder.build_return(None)?;
		if self.is_split() {
			self.export_symbol(RELEASE.to_string());
		}
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
		let call = self.builder.build_call(callee.value, &arguments, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		let returned = call.try_as_basic_value().basic();
		// the result and the error
		let (result, error) = match (errors.is_some(), callee.ret.has_value()) {
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
			self.call_runtime("fa_flush", &[])?;
			self.builder.build_return(Some(&status))?;
			self.builder.position_at_end(succeeded);
		}
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
				self.call_runtime("fa_flush", &[])?;
				self.builder.build_return(Some(&i32.const_zero()))?;
			}
			(None, Some(raw)) if c_ret != CType::Void => {
				let c_value = self.to_c(Val::new(callee.ret, raw), c_ret, node)?;
				self.call_runtime("fa_flush", &[])?;
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
					self.call_runtime("fa_flush", &[])?;
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
		let size = self
			.builder
			.build_int_add(length, self.usize.const_int(1, false), "")?;
		let block = self
			.builder
			.build_call(self.libc_function("malloc"), &[size.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("a block")
			.into_pointer_value();
		self.builder.build_memcpy(block, 1, bytes, 1, length)?;
		// SAFETY: the block holds `length + 1` bytes
		let end = unsafe {
			self.builder
				.build_in_bounds_gep(self.context.i8_type(), block, &[length], "")?
		};
		self.builder
			.build_store(end, self.context.i8_type().const_zero())?;
		let global = self.error_message_global().as_pointer_value();
		let previous = self.builder.build_load(self.pointer_type(), global, "")?;
		self.builder
			.build_call(self.libc_function("free"), &[previous.into()], "")?;
		self.builder.build_store(global, block)?;
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
			if let Some(release) = self.module.get_function(RELEASE) {
				self.builder.build_call(release, &[], "")?;
			}
			if self.leak_check {
				self.call_runtime("fa_leak_check", &[])?;
			}
			self.call_runtime("fa_flush", &[])?;
			self.builder.build_return(None)?;
		}
		self.end_function()?;
		add_structor(self.module, init, true);
		add_structor(self.module, fini, false);
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
