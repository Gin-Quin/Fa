//! Async natively (`design/async.md` §3, steps N1 and N2): `await`, `start`,
//! `start do`, tasks, `Task.all/any/settled`, `Task.yield`, `sleep`, and the
//! waits of mutexes and channels. The runtime part (the executor, the
//! timers, the lines of waiting tasks) is the "tasks and the event loop"
//! section of `runtime/fa_runtime.c`.
//!
//! - **Asynchronous mode** (`Program::asynchronous`, [`Tasks::asynchronous`]):
//!   off, the whole program is one blocking call (no coroutine, no
//!   executor): `start` runs the task to its end and gives a *finished
//!   block*, `await` reads it, `Task.yield()` does nothing, `sleep` sleeps.
//! - **Async instances**: a function awaited or started from an async
//!   instance gets a second LLVM function, `fa.<name>.async(params…, i32
//!   mode) -> ptr`, an LLVM switched-resume coroutine (`presplitcoroutine`)
//!   whose body is the same lowering as the synchronous instance's. Its
//!   frame comes from `fa_alloc(llvm.coro.size)`; its promise, aligned on
//!   16 bytes so that it is at 16 bytes in the frame on every target, is a
//!   `FaTask` header followed by the result (what the synchronous instance
//!   returns). `mode` bit 0: a started task waits at an initial suspension
//!   for its first turn; bit 1: a detached task owns its parameters.
//! - **Suspension points**: each has a destroy edge, which cancels the wait
//!   (a timer, a place in a line), then drops what the cleanup stack holds
//!   (`ownership.rs`), then frees the frame: dropping a task that isn't done
//!   cancels it at its suspension point.
//! - **`await f()`** of a function that may suspend, in an async instance:
//!   its async instance runs at once (`mode` 0); when its frame is done
//!   (`llvm.coro.done`) its result is read without the executor, else the
//!   caller waits for it (`fa_task_await`, its end resumes the caller
//!   through the trampoline). Elsewhere, and for a function that can't
//!   suspend, `await` is a plain call.
//! - **`start f()`**: the async instance at its initial suspension, queued
//!   (`fa_task_spawn`); detached (a statement), it owns copies of its
//!   arguments and the executor frees it. In a blocking call (the
//!   synchronous instance of an `async` function, or asynchronous mode
//!   off) the synchronous instance runs and the task is a finished block;
//!   in a function that isn't `async`, the thread's blocking depth decides.
//! - **The coop budget** (A2): every iteration of a loop of an async
//!   instance spends a unit (one load, a decrement, a branch); when it is
//!   spent and the turn lasted more than 2 ms, the task goes to the tail of
//!   the ready queue.

use fa_analyzer::analysis::{Argument, AwaitKind, CallInfo, FnParam, StartInfo};
use fa_analyzer::ids::{FileId, NodeId, SymbolId, TypeId};
use fa_analyzer::syntax::View;
use fa_analyzer::types::TypeKind;
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::intrinsics::Intrinsic;
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, StructType};
use inkwell::values::{
	AsValueRef, BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue,
	PointerValue,
};
use llvm_sys::core::{
	LLVMBuildCall2, LLVMConstNull, LLVMGlobalGetValueType, LLVMTokenTypeInContext,
};
use llvm_sys::prelude::LLVMValueRef;
use rustc_hash::FxHashMap;

use crate::error::Error;
use crate::lower::ownership::Cleanup;
use crate::lower::values::{IntRepr, Repr, Val};
use crate::lower::{FAST_CALLING_CONVENTION, Function, Lowering, Slot};
use crate::target::Target;

/// Where the promise (the `FaTask` header) is in a frame (`TASK_AT`).
const TASK_AT: u64 = 16;
/// The alignment of a promise: the same for `llvm.coro.id` and the frame.
const PROMISE_ALIGN: u32 = 16;
/// `TASK_RUNNING`, `TASK_DONE`, `TASK_RESULT` of the runtime.
const STATE_RUNNING: u64 = 4;
const STATE_DONE: u64 = 3;
const FLAG_RESULT: u64 = 2;
/// The words of a wait (`FaWait`) and of a join node (`FaJoin`).
pub(super) const WAIT_WORDS: u32 = 16;
const JOIN_WORDS: u32 = 8;
/// The results of `fa_*_async` and the statuses of a wait.
pub(super) const AWAIT_READY: u64 = 0;
const AWAIT_CLOSED: u64 = 3;
const WAIT_DONE: u64 = 1;
/// The bits of the `mode` of an async instance.
const MODE_LAZY: u64 = 1;
const MODE_OWNS: u64 = 2;

/// The async state of a lowering.
#[derive(Default)]
pub(crate) struct Tasks<'ctx> {
	/// The program runs in asynchronous mode (`Program::asynchronous`).
	pub(crate) asynchronous: bool,
	/// Dev profile: the event loop warns about long turns, and suspension
	/// points record where they are.
	pub(crate) dev: bool,
	/// The async instances, by the synchronous instance's function.
	instances: FxHashMap<FunctionValue<'ctx>, AsyncInstance<'ctx>>,
	/// The async instances to lower, by their synchronous function.
	pending: Vec<FunctionValue<'ctx>>,
	/// The async instance being lowered.
	lowering: Option<AsyncInstance<'ctx>>,
	/// The calls to lower as awaited or started calls.
	call_modes: FxHashMap<NodeId, CallMode>,
	/// The `Task.all`, `Task.any` and `Task.settled` coroutines made.
	joins: FxHashMap<String, FunctionValue<'ctx>>,
	/// The destroy functions of finished blocks, by result shape.
	finished: FxHashMap<String, FunctionValue<'ctx>>,
	/// The exit code of the entry function, set by the top level's coroutine.
	exit_status: Option<PointerValue<'ctx>>,
	/// The operand of an `await` lowered next, a task-returning expression
	/// that runs at once (A15).
	immediate: Option<NodeId>,
	/// The literal of a worker whose state is being built (`worker_literal`).
	worker_state: Option<NodeId>,
	/// The awaited calls lowered as plain calls (the callee can't
	/// suspend): not blocking calls (`blocking_starter`).
	awaited_plain: rustc_hash::FxHashSet<NodeId>,
}

/// An async instance of a function.
#[derive(Clone)]
struct AsyncInstance<'ctx> {
	function: Function<'ctx>,
	file: FileId,
	key: NodeId,
	/// Its loops spend the coop budget (an `async` function: A2).
	coop: bool,
	/// For the dev profile's messages.
	name: String,
}

/// How a call is lowered by [`Lowering::task_call`].
#[derive(Copy, Clone, Debug)]
enum CallMode {
	/// Awaited in an async instance: its async instance, then the await.
	Await,
	/// Started: a task of this type. `immediate`: awaited right away (A15),
	/// so it runs at once and isn't queued.
	Start {
		detached: bool,
		immediate: bool,
		task: TypeId,
	},
}

/// What [`Lowering::task_call`] made of a call.
pub(crate) enum TaskCall<'ctx> {
	/// A task (or nothing, for a detached task): the value of the call.
	Task(Val<'ctx>),
	/// The result of an awaited call, as the synchronous instance returns it.
	Result(Option<BasicValueEnum<'ctx>>),
}

/// What the function being lowered is, for tasks.
#[derive(Clone, Default)]
pub(crate) enum FrameTask<'ctx> {
	/// A function that isn't `async`: in an asynchronous program, `start`
	/// checks the thread's blocking depth.
	#[default]
	Plain,
	/// The synchronous instance of an `async` function: a blocking call,
	/// where `start` runs the task to its end. `counted`: it raised the
	/// blocking depth, lowered at its returns.
	Blocking { counted: bool },
	/// An async instance, or the top level of an asynchronous program.
	Coroutine(Coroutine<'ctx>),
}

/// A coroutine being lowered.
#[derive(Clone)]
pub(crate) struct Coroutine<'ctx> {
	/// The frame (`llvm.coro.begin`).
	pub(super) handle: PointerValue<'ctx>,
	/// The promise (an alloca given to `llvm.coro.id`) and its type.
	promise: PointerValue<'ctx>,
	promise_type: StructType<'ctx>,
	/// The result, as the synchronous instance returns it.
	ret: Repr,
	throws: bool,
	/// The end of the task: its final suspension.
	finish: BasicBlock<'ctx>,
	/// Frees the frame, then goes to `end`.
	cleanup: BasicBlock<'ctx>,
	/// `llvm.coro.end`, then returns the frame.
	end: BasicBlock<'ctx>,
	/// The `mode` parameter.
	mode: IntValue<'ctx>,
	/// Its loops spend the coop budget.
	coop: bool,
	/// The top level of the program (its end stores the exit code).
	top: bool,
	/// Its name, for the dev profile's messages.
	name: String,
}

impl<'ctx> Lowering<'_, 'ctx> {
	// ── the mode ──────────────────────────────────────────────────────────

	/// Sets the mode of the program (`Program::asynchronous`) and the dev
	/// profile (warnings about long turns).
	pub(crate) fn with_asynchronous(mut self, asynchronous: bool, dev: bool) -> Self {
		self.tasks.asynchronous = asynchronous;
		self.tasks.dev = dev;
		self
	}

	/// Whether the function being lowered is a coroutine.
	pub(crate) fn in_coroutine(&self) -> bool {
		matches!(self.frame().task, FrameTask::Coroutine(_))
	}

	/// Whether the function being lowered is the body of a task that isn't
	/// the top level (an async instance, a `start do`, a combinator).
	pub(crate) fn in_task_body(&self) -> bool {
		matches!(&self.frame().task, FrameTask::Coroutine(coroutine) if !coroutine.top)
	}

	pub(super) fn coroutine(&self) -> Option<Coroutine<'ctx>> {
		match &self.frame().task {
			FrameTask::Coroutine(coroutine) => Some(coroutine.clone()),
			_ => None,
		}
	}

	/// The type of what the function being lowered returns (in its
	/// promise, for a coroutine).
	pub(crate) fn raw_return_type(&self) -> Option<BasicTypeEnum<'ctx>> {
		match &self.frame().task {
			FrameTask::Coroutine(coroutine) => self.raw_type(coroutine.ret, coroutine.throws),
			_ => self.frame().function.get_type().get_return_type(),
		}
	}

	// ── LLVM's coroutine intrinsics ───────────────────────────────────────

	/// Calls an intrinsic with raw values: some take or give a `token`,
	/// which inkwell can't represent.
	fn coro_intrinsic(
		&self,
		name: &str,
		overloads: &[BasicTypeEnum<'ctx>],
		arguments: &[LLVMValueRef],
	) -> Result<LLVMValueRef, Error> {
		let declaration = Intrinsic::find(name)
			.and_then(|intrinsic| intrinsic.get_declaration(self.module, overloads))
			.ok_or_else(|| Error::Llvm(format!("no intrinsic {name}")))?;
		let mut arguments = arguments.to_vec();
		// SAFETY: the builder is positioned, and the arguments match the
		// intrinsic's signature
		Ok(unsafe {
			LLVMBuildCall2(
				self.builder.as_mut_ptr(),
				LLVMGlobalGetValueType(declaration.as_value_ref()),
				declaration.as_value_ref(),
				arguments.as_mut_ptr(),
				arguments.len() as u32,
				c"".as_ptr(),
			)
		})
	}

	fn token_none(&self) -> LLVMValueRef {
		// SAFETY: a constant of the context
		unsafe { LLVMConstNull(LLVMTokenTypeInContext(self.context.raw())) }
	}

	fn raw_bool(&self, value: bool) -> LLVMValueRef {
		self.context
			.bool_type()
			.const_int(u64::from(value), false)
			.as_value_ref()
	}

	/// `{ ptr parent, ptr next, ptr prev, i32 state, i32 flags }` (`FaTask`).
	fn task_header_type(&self) -> StructType<'ctx> {
		let pointer = self.pointer_type();
		let i32 = self.context.i32_type();
		self.context.struct_type(
			&[
				pointer.into(),
				pointer.into(),
				pointer.into(),
				i32.into(),
				i32.into(),
			],
			false,
		)
	}

	/// What a function of this result returns: its value, `{ value, ptr
	/// error }` when it throws, the error alone, or nothing.
	fn raw_type(&self, ret: Repr, throws: bool) -> Option<BasicTypeEnum<'ctx>> {
		match (ret.has_value(), throws) {
			(true, false) => Some(self.llvm_type(ret)),
			(false, false) => None,
			(true, true) => Some(
				self.context
					.struct_type(&[self.llvm_type(ret), self.pointer_type().into()], false)
					.into(),
			),
			(false, true) => Some(self.pointer_type().into()),
		}
	}

	/// The promise of a task of this result: its header, then its result,
	/// always with room for an error (null when the task can't fail), so
	/// that a `Task(T)` is a `Task(T) throws E` as it is: the same frame,
	/// read with either layout ([`Lowering::promise_raw_type`]).
	fn promise_type(&self, ret: Repr, _throws: bool) -> StructType<'ctx> {
		let fields: [BasicTypeEnum<'ctx>; 2] =
			[self.task_header_type().into(), self.promise_raw_type(ret)];
		self.context.struct_type(&fields, false)
	}

	/// The result in a promise: `{ value, error }`, or the error alone for a
	/// task without value, whether the task can fail or not.
	fn promise_raw_type(&self, ret: Repr) -> BasicTypeEnum<'ctx> {
		self.raw_type(ret, true).expect("a result with an error")
	}

	/// A raw result as a function returns it (`raw_type(ret, throws)`), as
	/// it is stored in a promise: with a null error when it can't fail.
	fn to_promise_raw(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		raw: Option<BasicValueEnum<'ctx>>,
		ret: Repr,
		throws: bool,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		let null = self.pointer_type().const_null();
		if throws {
			return Ok(raw.unwrap_or_else(|| null.into()));
		}
		if !ret.has_value() {
			return Ok(null.into());
		}
		let pair = self.promise_raw_type(ret).into_struct_type();
		let value = raw.unwrap_or_else(|| super::zero_of(self.llvm_type(ret)));
		let pair = builder.build_insert_value(pair.get_undef(), value, 0, "")?;
		Ok(builder
			.build_insert_value(pair, null, 1, "")?
			.as_basic_value_enum())
	}

	/// A result read from a promise, as a function returns it.
	fn raw_of_promise(
		&self,
		raw: BasicValueEnum<'ctx>,
		ret: Repr,
		throws: bool,
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		if throws {
			return Ok(Some(raw));
		}
		if !ret.has_value() {
			return Ok(None);
		}
		Ok(Some(self.builder.build_extract_value(
			raw.into_struct_value(),
			0,
			"",
		)?))
	}

	/// Loads the result of a promise, as a function returns it.
	fn load_promise_raw(
		&self,
		promise: PointerValue<'ctx>,
		ret: Repr,
		throws: bool,
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		let promise_type = self.promise_type(ret, throws);
		let field = self
			.builder
			.build_struct_gep(promise_type, promise, 1, "")?;
		if !throws {
			// the value alone: the error slot is never read
			if !ret.has_value() {
				return Ok(None);
			}
			let pair = self.promise_raw_type(ret).into_struct_type();
			let value = self.builder.build_struct_gep(pair, field, 0, "")?;
			return Ok(Some(self.builder.build_load(
				self.llvm_type(ret),
				value,
				"result",
			)?));
		}
		let raw = self
			.builder
			.build_load(self.promise_raw_type(ret), field, "result")?;
		self.raw_of_promise(raw, ret, throws)
	}

	/// The result of the tasks of a `Task(T) throws E` type.
	fn task_layout(&self, task: TypeId, node: NodeId) -> Result<(Repr, bool), Error> {
		match self.types().kind(task) {
			TypeKind::Task(value, error) => {
				let ret = self.repr_of(value, node.index())?;
				let ret = if ret == Repr::Never { Repr::Void } else { ret };
				Ok((ret, error != TypeId::NEVER))
			}
			_ => Err(self.unsupported(node.index(), "this task type")),
		}
	}

	/// The promise of a frame (or of a finished block).
	fn promise_of(&self, frame: PointerValue<'ctx>) -> Result<PointerValue<'ctx>, Error> {
		Ok(unsafe {
			self.builder.build_in_bounds_gep(
				self.context.i8_type(),
				frame,
				&[self.context.i64_type().const_int(TASK_AT, false)],
				"promise",
			)?
		})
	}

	/// A call of a runtime function with a result.
	pub(super) fn runtime_value(
		&self,
		name: &str,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<BasicValueEnum<'ctx>, Error> {
		self.call_value(self.runtime.function(name)?, arguments)
	}

	/// A thread-local integer of the runtime (`fa_coop_budget`, `fa_blocking_depth`).
	fn runtime_global(&self, name: &str) -> Result<PointerValue<'ctx>, Error> {
		self.module
			.get_global(name)
			.map(|global| global.as_pointer_value())
			.ok_or_else(|| Error::Llvm(format!("the runtime has no `{name}`")))
	}

	/// A NUL-terminated constant string.
	fn c_string(&mut self, text: &str) -> PointerValue<'ctx> {
		let mut bytes = text.as_bytes().to_vec();
		bytes.push(0);
		self.constant_bytes(&bytes)
	}

	/// A slot of `words` 64-bit words in the frame (a wait, a join node).
	pub(super) fn words(&self, words: u32, name: &str) -> Result<PointerValue<'ctx>, Error> {
		let ty = self.context.i64_type().array_type(words);
		self.alloca_type(ty.into(), name)
	}

	// ── coroutines ────────────────────────────────────────────────────────

	/// Makes the function being lowered (whose `allocas` and first block
	/// exist, the builder in the first block) a coroutine returning its
	/// frame: `llvm.coro.id` with the promise, the frame from `fa_alloc`,
	/// `llvm.coro.begin`, the header of the task; the cleanup and end
	/// blocks. The builder stays where the body goes.
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn begin_coroutine(
		&mut self,
		ret: Repr,
		throws: bool,
		mode: IntValue<'ctx>,
		coop: bool,
		top: bool,
		name: String,
	) -> Result<(), Error> {
		let function = self.frame().function;
		self.runtime
			.add_attributes(function, &["presplitcoroutine"]);
		let promise_type = self.promise_type(ret, throws);
		let promise = self.alloca_type(promise_type.into(), "promise")?;
		promise
			.as_instruction_value()
			.expect("an alloca")
			.set_alignment(PROMISE_ALIGN)
			.map_err(|error| Error::Llvm(error.to_string()))?;
		let i32 = self.context.i32_type();
		let pointer = self.pointer_type();
		let null = pointer.const_null();
		let id = self.coro_intrinsic(
			"llvm.coro.id",
			&[],
			&[
				i32.const_int(u64::from(PROMISE_ALIGN), false)
					.as_value_ref(),
				promise.as_value_ref(),
				null.as_value_ref(),
				null.as_value_ref(),
			],
		)?;
		let needed = self.coro_intrinsic("llvm.coro.alloc", &[], &[id])?;
		let entry = self.builder.get_insert_block().expect("a block");
		let allocate = self.append_block("coro_allocate");
		let begin = self.append_block("coro_begin");
		// SAFETY: `llvm.coro.alloc` gives an `i1`
		let needed = unsafe { IntValue::new(needed) };
		self.builder
			.build_conditional_branch(needed, allocate, begin)?;
		self.builder.position_at_end(allocate);
		let size = self.coro_intrinsic("llvm.coro.size", &[self.usize.into()], &[])?;
		let align = self.usize.const_int(u64::from(PROMISE_ALIGN), false);
		// SAFETY: `llvm.coro.size` gives a `usize`
		let size = unsafe { IntValue::new(size) };
		let memory = self
			.call_value(self.runtime.alloc()?, &[size.into(), align.into()])?
			.into_pointer_value();
		self.builder.build_unconditional_branch(begin)?;
		self.builder.position_at_end(begin);
		let phi = self.builder.build_phi(pointer, "memory")?;
		phi.add_incoming(&[(&null, entry), (&memory, allocate)]);
		let handle = self.coro_intrinsic(
			"llvm.coro.begin",
			&[],
			&[id, phi.as_basic_value().as_value_ref()],
		)?;
		// SAFETY: `llvm.coro.begin` gives a `ptr`
		let handle = unsafe { PointerValue::new(handle) };
		// the header: running, no parent
		let header = self.task_header_type();
		let header_value = header.const_named_struct(&[
			null.into(),
			null.into(),
			null.into(),
			i32.const_int(STATE_RUNNING, false).into(),
			i32.const_zero().into(),
		]);
		let header_pointer = self
			.builder
			.build_struct_gep(promise_type, promise, 0, "")?;
		self.builder.build_store(header_pointer, header_value)?;

		let body = self.builder.get_insert_block().expect("a block");
		let cleanup = self.append_block("coro_cleanup");
		let free = self.append_block("coro_free");
		let end = self.append_block("coro_end");
		let finish = self.append_block("coro_finish");
		self.builder.position_at_end(cleanup);
		let frame = self.coro_intrinsic("llvm.coro.free", &[], &[id, handle.as_value_ref()])?;
		// SAFETY: `llvm.coro.free` gives a `ptr`
		let frame = unsafe { PointerValue::new(frame) };
		let allocated = self.builder.build_is_not_null(frame, "")?;
		self.builder
			.build_conditional_branch(allocated, free, end)?;
		self.builder.position_at_end(free);
		let size = self.coro_intrinsic("llvm.coro.size", &[self.usize.into()], &[])?;
		// SAFETY: `llvm.coro.size` gives a `usize`
		let size = unsafe { IntValue::new(size) };
		self.builder.build_call(
			self.runtime.free()?,
			&[frame.into(), size.into(), align.into()],
			"",
		)?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		self.coro_intrinsic(
			"llvm.coro.end",
			&[],
			&[
				handle.as_value_ref(),
				self.raw_bool(false),
				self.token_none(),
			],
		)?;
		self.builder.build_return(Some(&handle))?;
		self.builder.position_at_end(body);
		self.frame_mut().task = FrameTask::Coroutine(Coroutine {
			handle,
			promise,
			promise_type,
			ret,
			throws,
			finish,
			cleanup,
			end,
			mode,
			coop,
			top,
			name,
		});
		Ok(())
	}

	/// The end of the coroutine being lowered: the block every return goes
	/// to (its result in the promise) wakes what awaits the task, then
	/// suspends for the last time; the destroy edge of that suspension
	/// drops the result when nothing took it.
	pub(crate) fn end_coroutine(&mut self) -> Result<(), Error> {
		let Some(coroutine) = self.coroutine() else {
			return Ok(());
		};
		self.builder.position_at_end(coroutine.finish);
		self.record_site(&format!("{}, at its end", coroutine.name))?;
		self.call_runtime("fa_task_finish", &[coroutine.handle.into()])?;
		let state = self.coro_intrinsic(
			"llvm.coro.suspend",
			&[],
			&[self.token_none(), self.raw_bool(true)],
		)?;
		// SAFETY: `llvm.coro.suspend` gives an `i8`
		let state = unsafe { IntValue::new(state) };
		let resumed = self.append_block("coro_resumed_after_end");
		let destroyed = self.append_block("coro_destroyed_after_end");
		let i8 = self.context.i8_type();
		self.builder.build_switch(
			state,
			coroutine.end,
			&[
				(i8.const_int(0, false), resumed),
				(i8.const_int(1, false), destroyed),
			],
		)?;
		self.builder.position_at_end(resumed);
		self.builder.build_unreachable()?;
		self.builder.position_at_end(destroyed);
		let result_drop = coroutine.ret.has_value() && self.needs_drop(coroutine.ret);
		if result_drop || coroutine.throws {
			let header =
				self.builder
					.build_struct_gep(coroutine.promise_type, coroutine.promise, 0, "")?;
			let promise = coroutine.promise;
			self.drop_promise_result(
				header,
				promise,
				coroutine.promise_type,
				coroutine.ret,
				coroutine.throws,
			)?;
		}
		self.builder.build_unconditional_branch(coroutine.cleanup)?;
		Ok(())
	}

	/// Drops the result in a promise unless it was taken (`TASK_RESULT`).
	fn drop_promise_result(
		&mut self,
		header: PointerValue<'ctx>,
		promise: PointerValue<'ctx>,
		promise_type: StructType<'ctx>,
		ret: Repr,
		throws: bool,
	) -> Result<(), Error> {
		let i32 = self.context.i32_type();
		let flags = self
			.builder
			.build_struct_gep(self.task_header_type(), header, 4, "")?;
		let flags = self.builder.build_load(i32, flags, "")?.into_int_value();
		let kept = self
			.builder
			.build_and(flags, i32.const_int(FLAG_RESULT, false), "")?;
		let kept = self
			.builder
			.build_int_compare(IntPredicate::NE, kept, i32.const_zero(), "")?;
		let drop = self.append_block("drop_result");
		let after = self.append_block("dropped_result");
		self.builder.build_conditional_branch(kept, drop, after)?;
		self.builder.position_at_end(drop);
		let _ = promise_type;
		if let Some(raw) = self.load_promise_raw(promise, ret, throws)? {
			self.drop_raw(raw, ret, throws)?;
		}
		self.builder.build_unconditional_branch(after)?;
		self.builder.position_at_end(after);
		Ok(())
	}

	/// Drops a raw result: its value and its error.
	fn drop_raw(&self, raw: BasicValueEnum<'ctx>, ret: Repr, throws: bool) -> Result<(), Error> {
		let (value, error) = match (ret.has_value(), throws) {
			(true, true) => {
				let pair = raw.into_struct_value();
				(
					Some(self.builder.build_extract_value(pair, 0, "")?),
					Some(self.builder.build_extract_value(pair, 1, "")?),
				)
			}
			(true, false) => (Some(raw), None),
			(false, true) => (None, Some(raw)),
			(false, false) => (None, None),
		};
		if let Some(value) = value
			&& self.needs_drop(ret)
		{
			self.emit_drop(Val::new(ret, value))?;
		}
		if let Some(error) = error {
			self.emit_drop(Val::new(Repr::Union, error))?;
		}
		Ok(())
	}

	/// In dev profiles, records where the running task is (for the warning
	/// about long turns).
	fn record_site(&mut self, site: &str) -> Result<(), Error> {
		if !self.tasks.dev {
			return Ok(());
		}
		let text = self.c_string(site);
		let global = self.runtime_global("fa_turn_site")?;
		self.builder.build_store(global, text)?;
		Ok(())
	}

	/// The site of a node, for messages: `name, file:line:column`.
	fn site_of(&self, node: NodeId) -> String {
		let name = self
			.coroutine()
			.map(|coroutine| coroutine.name)
			.unwrap_or_else(|| "main".to_string());
		format!("{name}, {}", self.location(node.index()))
	}

	/// A suspension point of the coroutine being lowered. Resumed, the code
	/// goes on after it; destroyed, `on_destroy` cancels what the task
	/// waits for, then the cleanup stack is dropped and the frame freed.
	pub(super) fn suspend(
		&mut self,
		node: Option<NodeId>,
		on_destroy: impl FnOnce(&mut Self) -> Result<(), Error>,
	) -> Result<(), Error> {
		let coroutine = self.coroutine().expect("a coroutine");
		if let Some(node) = node {
			let site = self.site_of(node);
			self.record_site(&site)?;
		}
		let state = self.coro_intrinsic(
			"llvm.coro.suspend",
			&[],
			&[self.token_none(), self.raw_bool(false)],
		)?;
		// SAFETY: `llvm.coro.suspend` gives an `i8`
		let state = unsafe { IntValue::new(state) };
		let resumed = self.append_block("resumed");
		let destroyed = self.append_block("cancelled");
		let i8 = self.context.i8_type();
		self.builder.build_switch(
			state,
			coroutine.end,
			&[
				(i8.const_int(0, false), resumed),
				(i8.const_int(1, false), destroyed),
			],
		)?;
		self.builder.position_at_end(destroyed);
		on_destroy(self)?;
		self.drop_to(0)?;
		self.branch_to(coroutine.cleanup)?;
		self.builder.position_at_end(resumed);
		Ok(())
	}

	/// Returns from the function being lowered: a value of what it returns
	/// (`raw`); a coroutine stores it in its promise and ends.
	pub(crate) fn return_raw(&mut self, raw: Option<BasicValueEnum<'ctx>>) -> Result<(), Error> {
		match self.frame().task.clone() {
			FrameTask::Coroutine(coroutine) => {
				// with a null error when it can't fail (`promise_type`)
				let raw =
					self.to_promise_raw(&self.builder, raw, coroutine.ret, coroutine.throws)?;
				let result = self.builder.build_struct_gep(
					coroutine.promise_type,
					coroutine.promise,
					1,
					"",
				)?;
				self.builder.build_store(result, raw)?;
				self.builder.build_unconditional_branch(coroutine.finish)?;
			}
			FrameTask::Blocking { counted: true } => {
				self.add_blocking_depth(-1)?;
				self.builder
					.build_return(raw.as_ref().map(|raw| raw as _))?;
			}
			_ => {
				self.builder
					.build_return(raw.as_ref().map(|raw| raw as _))?;
			}
		}
		Ok(())
	}

	/// Whether a call of `function` is a blocking call of an `async`
	/// function that starts tasks without suspending (not awaited nor
	/// started): the caller raises the blocking depth around it, so that
	/// its `start`s run at once.
	pub(crate) fn blocking_starter(&self, node: NodeId, function: &Function<'ctx>) -> bool {
		self.tasks.asynchronous
			&& self.is_async_function(function)
			&& !self.tasks.awaited_plain.contains(&node)
			&& !matches!(self.frame().task, FrameTask::Blocking { .. })
			&& self
				.analysis
				.async_summaries
				.get(&function.declaration)
				.is_some_and(|summary| summary.starts && !summary.suspends)
	}

	/// Adds to the thread's blocking depth.
	pub(crate) fn add_blocking_depth(&self, delta: i64) -> Result<(), Error> {
		let i32 = self.context.i32_type();
		let depth = self.runtime_global("fa_blocking_depth")?;
		let value = self.builder.build_load(i32, depth, "")?.into_int_value();
		let value = self
			.builder
			.build_int_add(value, i32.const_int(delta as u64, true), "")?;
		self.builder.build_store(depth, value)?;
		Ok(())
	}

	// ── functions ─────────────────────────────────────────────────────────

	/// The function `lower_function_body` lowers: the async instance being
	/// lowered, or the function of `key`.
	pub(crate) fn function_to_lower(&self, key: NodeId) -> Function<'ctx> {
		match &self.tasks.lowering {
			Some(instance) if instance.key == key => instance.function.clone(),
			_ => self.functions[&key].clone(),
		}
	}

	/// Whether a function declaration is `async`.
	fn is_async_function(&self, function: &Function<'ctx>) -> bool {
		function
			.info
			.sig
			.is_some_and(|sig| self.types().sig(sig).is_async)
	}

	/// At the start of a function's body: an async instance becomes a
	/// coroutine; the synchronous instance of an `async` function is a
	/// blocking call (it raises the blocking depth when it may start tasks
	/// in functions that aren't `async`).
	pub(crate) fn begin_function_task(&mut self, function: &Function<'ctx>) -> Result<(), Error> {
		if let Some(instance) = self.tasks.lowering.clone()
			&& instance.function.value == function.value
		{
			let mode = function
				.value
				.get_last_param()
				.expect("a mode")
				.into_int_value();
			return self.begin_coroutine(
				function.ret,
				function.throws,
				mode,
				instance.coop,
				false,
				instance.name,
			);
		}
		if !self.tasks.asynchronous || !self.is_async_function(function) {
			return Ok(());
		}
		let summary = self.analysis.async_summaries.get(&function.declaration);
		// one that starts tasks without suspending is called plain when
		// awaited (its tasks start later, `concurrency.md`), and its blocking
		// callers raise the blocking depth (`blocking_starter`)
		if summary.is_some_and(|summary| summary.starts && !summary.suspends) {
			return Ok(());
		}
		let counted = summary.is_some_and(|summary| summary.starts);
		if counted {
			self.add_blocking_depth(1)?;
		}
		self.frame_mut().task = FrameTask::Blocking { counted };
		Ok(())
	}

	/// After the parameters of an async instance are bound: a detached
	/// task owns them, and a started task waits for its first turn.
	pub(crate) fn initial_suspend(
		&mut self,
		params: &[FnParam],
		by_pointer: &[bool],
	) -> Result<(), Error> {
		let Some(coroutine) = self.coroutine() else {
			return Ok(());
		};
		let i32 = self.context.i32_type();
		let owns = self
			.builder
			.build_and(coroutine.mode, i32.const_int(MODE_OWNS, false), "")?;
		let owns = self
			.builder
			.build_int_compare(IntPredicate::NE, owns, i32.const_zero(), "")?;
		let flag = self.alloca(Repr::Bool, "owns_parameters")?;
		self.builder.build_store(flag.pointer, owns)?;
		for (index, param) in params.iter().enumerate() {
			if param.default.is_some() || by_pointer.get(index).copied().unwrap_or(false) {
				continue;
			}
			if let Some(&slot) = self.slots.get(&param.symbol)
				&& self.needs_drop(slot.repr)
				// a move-only parameter is owned already (moved in by the caller)
				&& !self.is_moved_parameter(param.symbol)
			{
				self.own_flagged(slot, flag.pointer);
			}
		}
		self.lazy_suspend(coroutine.mode, |_| Ok(()))
	}

	/// Suspends when `mode` says the task is started (lazy).
	fn lazy_suspend(
		&mut self,
		mode: IntValue<'ctx>,
		on_destroy: impl FnOnce(&mut Self) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i32 = self.context.i32_type();
		let lazy = self
			.builder
			.build_and(mode, i32.const_int(MODE_LAZY, false), "")?;
		let lazy = self
			.builder
			.build_int_compare(IntPredicate::NE, lazy, i32.const_zero(), "")?;
		let wait = self.append_block("started");
		let run = self.append_block("first_turn");
		self.builder.build_conditional_branch(lazy, wait, run)?;
		self.builder.position_at_end(wait);
		self.suspend(None, on_destroy)?;
		self.builder.build_unconditional_branch(run)?;
		self.builder.position_at_end(run);
		Ok(())
	}

	/// At the end of a function's body: the end of a coroutine.
	pub(crate) fn end_function_task(&mut self) -> Result<(), Error> {
		self.end_coroutine()
	}

	/// [`Lowering::async_instance`], for the units of a program compiled
	/// per unit (`units.rs`).
	pub(crate) fn async_instance_of(
		&mut self,
		function: &Function<'ctx>,
	) -> Result<FunctionValue<'ctx>, Error> {
		self.async_instance(function)
	}

	/// The async instance of a function, declared on first use and lowered
	/// later (`lower_next_async_instance`).
	fn async_instance(&mut self, function: &Function<'ctx>) -> Result<FunctionValue<'ctx>, Error> {
		if let Some(instance) = self.tasks.instances.get(&function.value) {
			return Ok(instance.function.value);
		}
		let sync_type = function.value.get_type();
		let mut params: Vec<BasicMetadataTypeEnum<'ctx>> = sync_type.get_param_types();
		params.push(self.context.i32_type().into());
		let ty = self.pointer_type().fn_type(&params, false);
		// a function of another unit (a program compiled per unit): its unit
		// defines its async instance
		if let Some(value) = self.remote_async_instance(function, ty) {
			return Ok(value);
		}
		let Some((file, key)) = self.function_key(function.value) else {
			return Err(Error::Llvm(
				"an async instance of a function that isn't lowered".into(),
			));
		};
		let sync_name = function.value.get_name().to_string_lossy().into_owned();
		let value =
			self.module
				.add_function(&format!("{sync_name}.async"), ty, Some(Linkage::Internal));
		value.set_call_conventions(FAST_CALLING_CONVENTION);
		self.runtime.add_attributes(value, &["nounwind"]);
		self.export_async_instance(&sync_name, value);
		let name = sync_name
			.strip_prefix("fa.")
			.unwrap_or(&sync_name)
			.to_string();
		let instance = AsyncInstance {
			function: Function {
				value,
				..function.clone()
			},
			file,
			key,
			coop: self.is_async_function(function),
			name,
		};
		self.tasks.instances.insert(function.value, instance);
		self.tasks.pending.push(function.value);
		Ok(value)
	}

	/// Lowers the next async instance requested; false when none is left.
	pub(crate) fn lower_next_async_instance(&mut self) -> Result<bool, Error> {
		let Some(sync) = self.tasks.pending.pop() else {
			return Ok(false);
		};
		let instance = self.tasks.instances[&sync].clone();
		let current = self.file;
		self.switch_file(instance.file);
		let key = instance.key;
		let previous = self.tasks.lowering.replace(instance);
		let result = self.lower_function(key);
		self.tasks.lowering = previous;
		self.switch_file(current);
		result?;
		Ok(true)
	}

	/// `fa_resume` and `fa_destroy`, which the runtime calls: what
	/// `llvm.coro.resume` and `llvm.coro.destroy` lower to, a `fastcc` call
	/// of the first or the second pointer of the frame (finished blocks have
	/// a destroy function too, also in synchronous mode).
	pub(crate) fn define_coroutine_shims(&self) -> Result<(), Error> {
		for (name, index) in [("fa_resume", 0), ("fa_destroy", 1)] {
			let Some(function) = self.module.get_function(name) else {
				continue;
			};
			if function.count_basic_blocks() > 0 {
				continue;
			}
			function.set_linkage(Linkage::Internal);
			self.runtime.add_attributes(function, &["nounwind"]);
			let builder = self.context.create_builder();
			builder.position_at_end(self.context.append_basic_block(function, "entry"));
			let frame = function
				.get_nth_param(0)
				.expect("a frame")
				.into_pointer_value();
			self.build_frame_call(&builder, frame, index)?;
			builder.build_return(None)?;
		}
		Ok(())
	}

	/// The drop of a task: `fa_task_drop` (a task not done is cancelled);
	/// in synchronous mode, a finished block's destroy function (no event
	/// loop). Null and 1 (cancelled) are no tasks.
	pub(crate) fn build_task_drop(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		frame: PointerValue<'ctx>,
	) -> Result<(), Error> {
		if self.tasks.asynchronous {
			builder.build_call(self.runtime.function("fa_task_drop")?, &[frame.into()], "")?;
			return Ok(());
		}
		let function = builder
			.get_insert_block()
			.and_then(|block| block.get_parent())
			.expect("a function");
		let address = builder.build_ptr_to_int(frame, self.usize, "")?;
		let task = builder.build_int_compare(
			IntPredicate::UGT,
			address,
			self.usize.const_int(1, false),
			"",
		)?;
		let destroy = self.context.append_basic_block(function, "destroy_task");
		let after = self.context.append_basic_block(function, "task_dropped");
		builder.build_conditional_branch(task, destroy, after)?;
		builder.position_at_end(destroy);
		self.build_frame_call(builder, frame, 1)?;
		builder.build_unconditional_branch(after)?;
		builder.position_at_end(after);
		Ok(())
	}

	/// Calls the resume (0) or destroy (1) function of a frame.
	fn build_frame_call(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		frame: PointerValue<'ctx>,
		index: u64,
	) -> Result<(), Error> {
		let pointer = self.pointer_type();
		let slot = unsafe {
			builder.build_in_bounds_gep(
				pointer,
				frame,
				&[self.usize.const_int(index, false)],
				"",
			)?
		};
		let target = builder.build_load(pointer, slot, "")?.into_pointer_value();
		let ty = self.context.void_type().fn_type(&[pointer.into()], false);
		let call = builder.build_indirect_call(ty, target, &[frame.into()], "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		Ok(())
	}

	// ── the top level ─────────────────────────────────────────────────────

	/// The top level of an asynchronous program: a coroutine, run by the
	/// event loop from `main` (`fa_run_main`), which then flushes, reports
	/// leaks and exits with the entry function's code.
	pub(crate) fn lower_async_main(&mut self) -> Result<(), Error> {
		self.lower_async_main_of(|this, top| this.lower_top_level(top), |_, _| Ok(()))
	}

	/// [`Lowering::lower_async_main`] with another body for the coroutine
	/// (`body`), and code run by `main` before it (`prepare`): the tests
	/// (`testing.rs`).
	pub(crate) fn lower_async_main_of(
		&mut self,
		body: impl FnOnce(&mut Self, FunctionValue<'ctx>) -> Result<(), Error>,
		prepare: impl FnOnce(&mut Self, FunctionValue<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i32 = self.context.i32_type();
		let top_type = self.pointer_type().fn_type(&[i32.into()], false);
		let top = self
			.module
			.add_function("fa.top_level", top_type, Some(Linkage::Internal));
		top.set_call_conventions(FAST_CALLING_CONVENTION);
		self.runtime.add_attributes(top, &["nounwind"]);
		self.begin_function(top, Repr::Void, None);
		let mode = top.get_nth_param(0).expect("a mode").into_int_value();
		// its loops spend the coop budget too (design/decisions.md, top-level
		// back-edges)
		self.begin_coroutine(Repr::Void, false, mode, true, true, "main".to_string())?;
		body(self, top)?;
		self.end_coroutine()?;
		self.end_function()?;

		let main = self.declare_main();
		self.begin_function(main, Repr::Void, None);
		self.start_program(main)?;
		prepare(self, main)?;
		let call = self
			.builder
			.build_call(top, &[i32.const_zero().into()], "root")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		let root = call
			.try_as_basic_value()
			.basic()
			.expect("a frame")
			.into_pointer_value();
		let dev = i32.const_int(u64::from(self.tasks.dev), false);
		if self.target == Target::Wasm {
			return self.wasm_event_loop(root, dev);
		}
		self.call_runtime("fa_run_main", &[root.into(), dev.into()])?;
		self.call_runtime("fa_flush", &[])?;
		if self.leak_check {
			self.call_runtime("fa_leak_check", &[])?;
		}
		if let Some(status) = self.tasks.exit_status {
			let code = self.builder.build_load(i32, status, "")?;
			let exit = self.runtime.exit()?;
			self.builder.build_call(exit, &[code.into()], "")?;
		}
		self.builder.build_return(Some(&i32.const_zero()))?;
		self.end_function()
	}

	/// The end of `_start` on wasm (step W1): the host's event loop drives
	/// the executor. `_start` gives the root to the runtime and returns;
	/// `fa_run` (exported) runs a batch of the event loop, `fa_wake`
	/// (exported) queues the task of a wait the host completed, and
	/// `fa.finish` (the end of `main`: flush, leak check, exit code) runs
	/// once the root and the detached tasks are done.
	fn wasm_event_loop(
		&mut self,
		root: PointerValue<'ctx>,
		dev: IntValue<'ctx>,
	) -> Result<(), Error> {
		let i32 = self.context.i32_type();
		let void = self.context.void_type();
		let start = self.builder.get_insert_block().expect("a block");
		// the end of `main`
		let finish = self.module.add_function(
			"fa.finish",
			void.fn_type(&[], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(finish, &["nounwind"]);
		let entry = self.context.append_basic_block(finish, "entry");
		self.builder.position_at_end(entry);
		self.call_runtime("fa_flush", &[])?;
		if self.leak_check {
			self.call_runtime("fa_leak_check", &[])?;
		}
		if let Some(status) = self.tasks.exit_status {
			let code = self.builder.build_load(i32, status, "")?;
			let exit = self.runtime.exit()?;
			self.builder.build_call(exit, &[code.into()], "")?;
		}
		self.builder.build_return(None)?;
		// the exports of the loader
		let exported = |name: &str, function: FunctionValue<'ctx>| {
			function.add_attribute(
				inkwell::attributes::AttributeLoc::Function,
				self.context
					.create_string_attribute("wasm-export-name", name),
			);
			self.runtime.add_attributes(function, &["nounwind"]);
		};
		let run =
			self.module
				.add_function("fa_run", i32.fn_type(&[], false), Some(Linkage::External));
		exported("fa_run", run);
		let entry = self.context.append_basic_block(run, "entry");
		self.builder.position_at_end(entry);
		let batch = self.runtime.function("fa_wasm_run")?;
		let status = self
			.builder
			.build_call(batch, &[], "")?
			.try_as_basic_value()
			.basic()
			.expect("a status");
		self.builder.build_return(Some(&status))?;
		let pointer = self.pointer_type();
		let wake = self.module.add_function(
			"fa_wake",
			void.fn_type(&[pointer.into()], false),
			Some(Linkage::External),
		);
		exported("fa_wake", wake);
		let entry = self.context.append_basic_block(wake, "entry");
		self.builder.position_at_end(entry);
		let token = wake.get_nth_param(0).expect("a token");
		self.call_runtime("fa_wasm_wake", &[token.into()])?;
		self.builder.build_return(None)?;
		// `_start`
		self.builder.position_at_end(start);
		let finish = finish.as_global_value().as_pointer_value();
		self.call_runtime("fa_wasm_start", &[root.into(), dev.into(), finish.into()])?;
		self.builder.build_return(None)?;
		self.end_function()
	}

	/// At the end of the top level's coroutine: the exit code of the entry
	/// function, for `main`.
	pub(crate) fn store_exit_status(&mut self) -> Result<(), Error> {
		let Some(code) = self.exit_code else {
			return Ok(());
		};
		let i32 = self.context.i32_type();
		let status = match self.tasks.exit_status {
			Some(status) => status,
			None => {
				let global = self.module.add_global(i32, None, "fa.exit_status");
				global.set_linkage(Linkage::Internal);
				global.set_initializer(&i32.const_zero());
				let pointer = global.as_pointer_value();
				self.tasks.exit_status = Some(pointer);
				pointer
			}
		};
		let value = self.builder.build_load(i32, code, "")?;
		self.builder.build_store(status, value)?;
		Ok(())
	}

	/// The call of the entry function: awaited by the top level of an
	/// asynchronous program when it may suspend.
	pub(crate) fn entry_call(
		&mut self,
		function: &Function<'ctx>,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		let suspends = self
			.analysis
			.async_summaries
			.get(&function.declaration)
			.is_some_and(|summary| summary.suspends);
		if !(self.in_coroutine() && suspends) {
			let call = self.builder.build_call(function.value, arguments, "")?;
			call.set_call_convention(FAST_CALLING_CONVENTION);
			return Ok(call.try_as_basic_value().basic());
		}
		let instance = self.async_instance(function)?;
		let frame = self.call_async(instance, arguments, 0)?;
		self.await_frame(frame, None)?;
		self.take_result(frame, function.ret, function.throws)
	}

	// ── tasks ─────────────────────────────────────────────────────────────

	/// Calls an async instance with a mode: its frame.
	fn call_async(
		&self,
		instance: FunctionValue<'ctx>,
		arguments: &[BasicMetadataValueEnum<'ctx>],
		mode: u64,
	) -> Result<PointerValue<'ctx>, Error> {
		let mut arguments = arguments.to_vec();
		arguments.push(self.context.i32_type().const_int(mode, false).into());
		let call = self.builder.build_call(instance, &arguments, "task")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		Ok(call
			.try_as_basic_value()
			.basic()
			.expect("a frame")
			.into_pointer_value())
	}

	/// Whether a frame is done (`llvm.coro.done`: its resume pointer is null).
	fn frame_done(&self, frame: PointerValue<'ctx>) -> Result<IntValue<'ctx>, Error> {
		let resume = self
			.builder
			.build_load(self.pointer_type(), frame, "resume")?
			.into_pointer_value();
		Ok(self.builder.build_is_null(resume, "done")?)
	}

	/// Waits until a task is done, in the coroutine being lowered: at once
	/// when it is, else at a suspension point whose destroy edge drops it.
	fn await_frame(
		&mut self,
		frame: PointerValue<'ctx>,
		node: Option<NodeId>,
	) -> Result<(), Error> {
		let coroutine = self.coroutine().expect("a coroutine");
		let done = self.frame_done(frame)?;
		let wait = self.append_block("await");
		let ready = self.append_block("awaited");
		self.builder.build_conditional_branch(done, ready, wait)?;
		self.builder.position_at_end(wait);
		self.call_runtime("fa_task_await", &[frame.into(), coroutine.handle.into()])?;
		self.suspend(node, |this| {
			this.call_runtime("fa_task_drop", &[frame.into()])
		})?;
		self.builder.build_unconditional_branch(ready)?;
		self.builder.position_at_end(ready);
		Ok(())
	}

	/// Takes the result out of a done task (as its function returns it),
	/// then frees the task.
	fn take_result(
		&mut self,
		frame: PointerValue<'ctx>,
		ret: Repr,
		throws: bool,
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		let promise = self.promise_of(frame)?;
		let raw = self.load_promise_raw(promise, ret, throws)?;
		self.clear_result_flag(promise)?;
		self.build_frame_call(&self.builder, frame, 1)?;
		Ok(raw)
	}

	/// Marks the result of a task taken (its drop leaves it).
	fn clear_result_flag(&self, promise: PointerValue<'ctx>) -> Result<(), Error> {
		let i32 = self.context.i32_type();
		let flags = self
			.builder
			.build_struct_gep(self.task_header_type(), promise, 4, "")?;
		let value = self.builder.build_load(i32, flags, "")?.into_int_value();
		let value =
			self.builder
				.build_and(value, i32.const_int(!FLAG_RESULT & 0xFFFF_FFFF, false), "")?;
		self.builder.build_store(flags, value)?;
		Ok(())
	}

	/// The value of a raw result: its error raised, its value owned by the
	/// statement.
	fn raw_to_value(
		&mut self,
		raw: Option<BasicValueEnum<'ctx>>,
		ret: Repr,
		throws: bool,
	) -> Result<Val<'ctx>, Error> {
		let value = if throws {
			let Some(raw) = raw else {
				return Ok(Val::void());
			};
			let (value, error) = if ret.has_value() {
				let pair = raw.into_struct_value();
				(
					Some(self.builder.build_extract_value(pair, 0, "")?),
					self.builder
						.build_extract_value(pair, 1, "")?
						.into_pointer_value(),
				)
			} else {
				(None, raw.into_pointer_value())
			};
			self.raise_if_error(error)?;
			value
		} else {
			raw
		};
		Ok(match value {
			Some(value) if ret.has_value() => self.fresh(Val::new(ret, value)),
			_ => Val::void(),
		})
	}

	/// A raw result made of a value (`throws`: and no error).
	fn raw_of_value(
		&self,
		value: Val<'ctx>,
		ret: Repr,
		throws: bool,
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		let Some(raw_type) = self.raw_type(ret, throws) else {
			return Ok(None);
		};
		if !throws {
			return Ok(value.value.or_else(|| Some(super::zero_of(raw_type))));
		}
		if !ret.has_value() {
			return Ok(Some(self.pointer_type().const_null().into()));
		}
		let pair = raw_type.into_struct_type();
		let inner = value
			.value
			.unwrap_or_else(|| super::zero_of(self.llvm_type(ret)));
		let pair = self
			.builder
			.build_insert_value(pair.get_undef(), inner, 0, "")?;
		let pair =
			self.builder
				.build_insert_value(pair, self.pointer_type().const_null(), 1, "")?;
		Ok(Some(pair.as_basic_value_enum()))
	}

	/// A raw result made of an error.
	fn raw_of_error(
		&self,
		error: PointerValue<'ctx>,
		ret: Repr,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		if !ret.has_value() {
			return Ok(error.into());
		}
		let pair = self.raw_type(ret, true).expect("a pair").into_struct_type();
		let pair = self.builder.build_insert_value(
			pair.get_undef(),
			super::zero_of(self.llvm_type(ret)),
			0,
			"",
		)?;
		let pair = self.builder.build_insert_value(pair, error, 1, "")?;
		Ok(pair.as_basic_value_enum())
	}

	/// A finished block holding a raw result: a task done from the start
	/// (a task of a blocking call, A13), with the layout of a frame.
	fn finished_block(
		&mut self,
		raw: Option<BasicValueEnum<'ctx>>,
		ret: Repr,
		throws: bool,
	) -> Result<PointerValue<'ctx>, Error> {
		let promise_type = self.promise_type(ret, throws);
		let size = self.finished_size(promise_type);
		let block = self.allocate(size)?;
		let destroy = self.finished_destroy(ret, throws)?;
		let pointer = self.pointer_type();
		self.builder.build_store(block, pointer.const_null())?;
		let destroy_slot = unsafe {
			self.builder.build_in_bounds_gep(
				pointer,
				block,
				&[self.usize.const_int(1, false)],
				"",
			)?
		};
		self.builder
			.build_store(destroy_slot, destroy.as_global_value().as_pointer_value())?;
		let promise = self.promise_of(block)?;
		let i32 = self.context.i32_type();
		let header = self.task_header_type().const_named_struct(&[
			pointer.const_null().into(),
			pointer.const_null().into(),
			pointer.const_null().into(),
			i32.const_int(STATE_DONE, false).into(),
			i32.const_int(FLAG_RESULT, false).into(),
		]);
		let header_pointer = self
			.builder
			.build_struct_gep(promise_type, promise, 0, "")?;
		self.builder.build_store(header_pointer, header)?;
		let raw = self.to_promise_raw(&self.builder, raw, ret, throws)?;
		let result = self
			.builder
			.build_struct_gep(promise_type, promise, 1, "")?;
		self.builder.build_store(result, raw)?;
		Ok(block)
	}

	/// The size of a finished block: the two pointers of a frame's header,
	/// padded to 16 bytes, then the promise.
	fn finished_size(&self, promise_type: StructType<'ctx>) -> IntValue<'ctx> {
		let size = promise_type.size_of().expect("a sized promise");
		let size = self
			.builder
			.build_int_cast(size, self.usize, "")
			.expect("a size");
		self.builder
			.build_int_add(size, self.usize.const_int(TASK_AT, false), "")
			.expect("a size")
	}

	/// The destroy function of the finished blocks of a result shape
	/// (called by `llvm.coro.destroy`, so `fastcc`): drops the result when
	/// it wasn't taken, then frees the block.
	fn finished_destroy(&mut self, ret: Repr, throws: bool) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!("{ret:?}.{throws}.{:?}", self.bindings.borrow().len());
		if let Some(function) = self.tasks.finished.get(&key) {
			return Ok(*function);
		}
		let pointer = self.pointer_type();
		let function = self.module.add_function(
			"fa.finished_task.destroy",
			self.context.void_type().fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		function.set_call_conventions(FAST_CALLING_CONVENTION);
		self.runtime.add_attributes(function, &["nounwind"]);
		self.tasks.finished.insert(key, function);
		let resume = self.builder.get_insert_block().expect("a block");
		let outer = self.frame.take();
		self.begin_function(
			function,
			Repr::Void,
			outer.as_ref().and_then(|frame| frame.owner),
		);
		let result = (|| -> Result<(), Error> {
			let block = function
				.get_nth_param(0)
				.expect("a block")
				.into_pointer_value();
			let promise_type = self.promise_type(ret, throws);
			let promise = self.promise_of(block)?;
			if self.raw_type(ret, throws).is_some() && (self.needs_drop(ret) || throws) {
				let header = self
					.builder
					.build_struct_gep(promise_type, promise, 0, "")?;
				self.drop_promise_result(header, promise, promise_type, ret, throws)?;
			}
			let size = self.finished_size(promise_type);
			let align = self.usize.const_int(u64::from(PROMISE_ALIGN), false);
			self.builder.build_call(
				self.runtime.free()?,
				&[block.into(), size.into(), align.into()],
				"",
			)?;
			self.builder.build_return(None)?;
			Ok(())
		})();
		let ended = self.end_function();
		self.frame = outer;
		self.builder.position_at_end(resume);
		result?;
		ended?;
		Ok(function)
	}

	/// How `start` runs here: `Some(true)` lazily (a task queued), `Some(false)`
	/// at once (a finished block), `None` as the blocking depth says.
	fn start_kind(&self) -> Option<bool> {
		if !self.tasks.asynchronous {
			return Some(false);
		}
		match self.frame().task {
			FrameTask::Coroutine(_) => Some(true),
			FrameTask::Blocking { .. } => Some(false),
			FrameTask::Plain => None,
		}
	}

	/// Branches on the blocking depth: `eager` builds the task of a
	/// blocking call, `lazy` a queued one; the task they give, or nothing.
	fn by_blocking_depth(
		&mut self,
		eager: impl FnOnce(&mut Self) -> Result<Option<PointerValue<'ctx>>, Error>,
		lazy: impl FnOnce(&mut Self) -> Result<Option<PointerValue<'ctx>>, Error>,
	) -> Result<Option<PointerValue<'ctx>>, Error> {
		let i32 = self.context.i32_type();
		let depth = self.runtime_global("fa_blocking_depth")?;
		let depth = self.builder.build_load(i32, depth, "")?.into_int_value();
		let blocking =
			self.builder
				.build_int_compare(IntPredicate::SGT, depth, i32.const_zero(), "")?;
		let eager_block = self.append_block("start_blocking");
		let lazy_block = self.append_block("start_task");
		let join = self.append_block("started");
		self.builder
			.build_conditional_branch(blocking, eager_block, lazy_block)?;
		self.builder.position_at_end(eager_block);
		let first = eager(self)?;
		let first_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(lazy_block);
		let second = lazy(self)?;
		let second_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		Ok(match (first, second) {
			(Some(first), Some(second)) => {
				let phi = self.builder.build_phi(self.pointer_type(), "task")?;
				phi.add_incoming(&[(&first, first_end), (&second, second_end)]);
				Some(phi.as_basic_value().into_pointer_value())
			}
			_ => None,
		})
	}

	/// An awaited or started call of a function (`calls.rs`), with its
	/// arguments: its async instance, or `None` for a plain call.
	pub(crate) fn task_call(
		&mut self,
		node: NodeId,
		function: &Function<'ctx>,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<Option<TaskCall<'ctx>>, Error> {
		let Some(mode) = self.tasks.call_modes.remove(&node) else {
			return Ok(None);
		};
		match mode {
			CallMode::Await => {
				let instance = self.async_instance(function)?;
				let frame = self.call_async(instance, arguments, 0)?;
				self.await_frame(frame, Some(node))?;
				let raw = self.take_result(frame, function.ret, function.throws)?;
				Ok(Some(TaskCall::Result(raw)))
			}
			CallMode::Start {
				detached,
				immediate,
				task,
			} => {
				let (ret, throws) = (function.ret, function.throws);
				// a detached task owns copies of its arguments
				let mut owned = arguments.to_vec();
				if detached {
					let mut position = usize::from(function.lambda)
						+ usize::from(function.return_slot)
						+ 2 * usize::from(function.return_collection);
					for (index, (repr, has_default)) in function.params.iter().enumerate() {
						let by_pointer = function.by_pointer.get(index).copied().unwrap_or(false);
						if !by_pointer && !has_default && self.needs_drop(*repr) {
							let value: Option<BasicValueEnum<'ctx>> = match owned.get(position) {
								Some(BasicMetadataValueEnum::PointerValue(value)) => {
									Some((*value).into())
								}
								Some(BasicMetadataValueEnum::StructValue(value)) => {
									Some((*value).into())
								}
								_ => None,
							};
							if let Some(value) = value {
								let copy = self.emit_copy(Val::new(*repr, value))?;
								owned[position] = copy.value.expect("a copy").into();
							}
						}
						position += 1 + usize::from(*has_default);
					}
				}
				let eager = |this: &mut Self| -> Result<Option<PointerValue<'ctx>>, Error> {
					let call = this.builder.build_call(function.value, arguments, "")?;
					call.set_call_convention(FAST_CALLING_CONVENTION);
					let raw = call.try_as_basic_value().basic();
					let raw = if function.return_slot {
						let BasicMetadataValueEnum::PointerValue(pointer) =
							arguments[usize::from(function.lambda)]
						else {
							unreachable!("return slot")
						};
						Some(this.raw_from_return_slot(
							&this.builder,
							Slot { pointer, repr: ret },
							raw,
						)?)
					} else {
						raw
					};
					if detached {
						if let Some(raw) = raw {
							this.drop_raw(raw, ret, throws)?;
						}
						return Ok(None);
					}
					Ok(Some(this.finished_block(raw, ret, throws)?))
				};
				let lazy = |this: &mut Self| -> Result<Option<PointerValue<'ctx>>, Error> {
					let instance = this.async_instance(function)?;
					let mode = if immediate {
						0
					} else if detached {
						MODE_LAZY | MODE_OWNS
					} else {
						MODE_LAZY
					};
					let arguments = if detached { &owned } else { arguments };
					let frame = this.call_async(instance, arguments, mode)?;
					if !immediate {
						let detached = this
							.context
							.i32_type()
							.const_int(u64::from(detached), false);
						this.call_runtime("fa_task_spawn", &[frame.into(), detached.into()])?;
					}
					Ok((!detached).then_some(frame))
				};
				let frame = match self.start_kind() {
					Some(true) => lazy(self)?,
					Some(false) => eager(self)?,
					None => self.by_blocking_depth(eager, lazy)?,
				};
				Ok(Some(TaskCall::Task(match frame {
					Some(frame) => self.fresh(Val::new(Repr::Task(task), frame)),
					None => Val::void(),
				})))
			}
		}
	}

	// ── await ─────────────────────────────────────────────────────────────

	/// The expression inside groups.
	pub(crate) fn ungrouped(&self, mut node: NodeId) -> NodeId {
		while let View::Group(inner) = self.view(node) {
			node = inner;
		}
		node
	}

	/// `await operand`.
	pub(crate) fn await_expression(
		&mut self,
		node: NodeId,
		operand: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some(info) = self.analysis.awaits.get(&node).cloned() else {
			return Err(self.unsupported(node.index(), "this `await`"));
		};
		let operand = self.ungrouped(operand);
		let waits = self.tasks.asynchronous && self.in_coroutine();
		match info.kind {
			AwaitKind::Call => {
				if waits && info.suspends && matches!(self.view(operand), View::Call { .. }) {
					self.tasks.call_modes.insert(operand, CallMode::Await);
				}
				// awaited, a call that can't suspend is a plain call, not a
				// blocking one
				self.tasks.awaited_plain.insert(operand);
				let value = self.expr(operand);
				self.tasks.awaited_plain.remove(&operand);
				self.tasks.call_modes.remove(&operand);
				value
			}
			// a call through a function value: its async entry (async_value_call)
			AwaitKind::FunctionValue => {
				if waits && matches!(self.view(operand), View::Call { .. }) {
					self.tasks.call_modes.insert(operand, CallMode::Await);
				}
				let value = self.expr(operand);
				self.tasks.call_modes.remove(&operand);
				value
			}
			AwaitKind::Primitive(name) if waits => self.await_primitive(node, name, operand),
			AwaitKind::Primitive(_) => self.expr(operand),
			AwaitKind::Task => {
				let task_type = self.type_of(operand);
				let (ret, throws) = self.task_layout(task_type, operand)?;
				let value = if info.immediate_task {
					self.immediate_task(operand)?
				} else {
					self.expr(operand)?
				};
				if value.repr == Repr::Never {
					return Ok(value);
				}
				let (frame, owned) = self.take_task(value, operand)?;
				if info.cancelled {
					self.raise_if_cancelled(frame)?;
				}
				if waits {
					self.await_frame(frame, Some(node))?;
				} else {
					// a blocking call: a finished block (A13)
					let done = self.frame_done(frame)?;
					let not_done = self.builder.build_not(done, "")?;
					self.guard(not_done, |this| this.call_noreturn("fa_task_stuck", &[]))?;
				}
				let raw = if owned {
					self.take_result(frame, ret, throws)?
				} else {
					// a parameter's task: its result taken, the frame left to
					// the caller, which drops it
					let promise = self.promise_of(frame)?;
					let raw = self.load_promise_raw(promise, ret, throws)?;
					self.clear_result_flag(promise)?;
					raw
				};
				self.raw_to_value(raw, ret, throws)
			}
		}
	}

	/// The task-returning operand of an `await` (A15): `start f()` runs at
	/// once, `Task.all(…)` too, without the queue.
	fn immediate_task(&mut self, operand: NodeId) -> Result<Val<'ctx>, Error> {
		let previous = self.tasks.immediate.replace(operand);
		let value = self.expr(operand);
		self.tasks.immediate = previous;
		value
	}

	/// Takes a task value out of what holds it (an `await` consumes it), and
	/// whether the `await` owns its frame now: a temporary, a local, a field
	/// or an element (the place gives it up: null), not a parameter (its
	/// caller's place keeps it, and drops it).
	fn take_task(
		&mut self,
		value: Val<'ctx>,
		operand: NodeId,
	) -> Result<(PointerValue<'ctx>, bool), Error> {
		let frame = value.value.expect("a task").into_pointer_value();
		self.give_task(value, operand)?;
		Ok((frame, true))
	}

	/// A task moves out of what holds it (`concurrency.md`: a task can't be
	/// copied): a temporary is unregistered, a place (a local, a parameter,
	/// a field, an item) gives it up (null, which its drop skips). For an
	/// `await`, an argument (the callee owns its task parameters), a
	/// returned or stored value.
	pub(crate) fn give_task(
		&mut self,
		value: Val<'ctx>,
		operand: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let mut inner = operand;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		// a task, a resource, a type holding one (`TypeStore::owns_unique`)
		if !self.types().owns_unique(self.type_of(inner)) {
			return Ok(value);
		}
		if !self.is_temporary(value)
			&& self.is_place(inner)
			&& let Ok(slot) = self.place(inner)
			&& slot.repr == value.repr
		{
			self.give_up_place(inner, slot)?;
			return Ok(value);
		}
		self.take(value, Some(operand))
	}

	/// Raises `Cancelled` when the task was cancelled (it is 1).
	fn raise_if_cancelled(&mut self, frame: PointerValue<'ctx>) -> Result<(), Error> {
		let address = self.builder.build_ptr_to_int(frame, self.usize, "")?;
		let cancelled = self.builder.build_int_compare(
			IntPredicate::EQ,
			address,
			self.usize.const_int(1, false),
			"",
		)?;
		let error = self.builtin_error("Cancelled");
		self.guard(cancelled, |this| this.raise(error))
	}

	/// A constant box of a built-in error without data (`Cancelled`).
	fn builtin_error(&self, name: &str) -> PointerValue<'ctx> {
		let id = fa_analyzer::builtins::errors::ERRORS
			.iter()
			.position(|error| error.name == name)
			.map_or(0, |index| index as u64 + 1);
		let i64 = self.context.i64_type();
		let ty = self.context.struct_type(&[i64.into()], false);
		let global = self.module.add_global(ty, None, "fa.error");
		global.set_initializer(&ty.const_named_struct(&[i64.const_int(id, false).into()]));
		global.set_constant(true);
		global.set_linkage(Linkage::Private);
		global.as_pointer_value()
	}

	// ── start ─────────────────────────────────────────────────────────────

	/// `start f()`, `start do { … }`.
	pub(crate) fn start_expression(
		&mut self,
		node: NodeId,
		operand: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some(info) = self.analysis.starts.get(&node).cloned() else {
			return Err(self.unsupported(node.index(), "this `start`"));
		};
		let immediate = self.tasks.immediate == Some(node);
		self.tasks.immediate = None;
		let task = self.type_of(node);
		if let Some(block) = info.block {
			return self.start_do(node, block, &info, immediate);
		}
		let call = self.ungrouped(operand);
		if !matches!(self.view(call), View::Call { .. }) {
			return Err(self.unsupported(node.index(), "starting this expression"));
		}
		self.tasks.call_modes.insert(
			call,
			CallMode::Start {
				detached: info.detached,
				immediate,
				task,
			},
		);
		let value = self.expr(call)?;
		if self.tasks.call_modes.remove(&call).is_none() || value.repr == Repr::Never {
			return Ok(value);
		}
		// not a direct call of a function (a function value, a built-in):
		// it ran at once, its value is the result of a finished task
		if info.detached {
			return Ok(Val::void());
		}
		let (ret, throws) = self.task_layout(task, node)?;
		let owned = if value.repr.has_value() {
			self.owned_as(value, ret, Some(call))?
		} else {
			value
		};
		let raw = self.raw_of_value(owned, ret, throws)?;
		let block = self.finished_block(raw, ret, throws)?;
		Ok(self.fresh(Val::new(Repr::Task(task), block)))
	}

	/// The locals a `start do` uses, declared outside of it (the
	/// module-level ones are globals), and their places.
	fn start_captures(
		&mut self,
		block: NodeId,
		node: NodeId,
	) -> Result<Vec<(SymbolId, Slot<'ctx>)>, Error> {
		use fa_analyzer::symbols::{Resolution, SymbolKind};
		let span = self.tree.spans[block.index()];
		let mut symbols = Vec::new();
		let mut stack = vec![block.index()];
		while let Some(current) = stack.pop() {
			if let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(current)) {
				let data = self.analysis.symbols.symbol(symbol);
				let variable = !matches!(
					data.kind,
					SymbolKind::Function
						| SymbolKind::Type | SymbolKind::Namespace
						| SymbolKind::TypeParameter
						| SymbolKind::Test
				);
				let declared = self.tree.spans[data.decl_node.index()];
				let outside = declared.start < span.start || declared.start >= span.end;
				if variable && outside && !symbols.contains(&symbol) {
					symbols.push(symbol);
				}
			}
			self.tree.nodes[current].for_each_child(|child| stack.push(child));
		}
		symbols.sort();
		let mut captures = Vec::new();
		for symbol in symbols {
			if self.shared.contains_key(&symbol) || self.global_slots.contains_key(&symbol) {
				continue;
			}
			let slot = self.slot_of(symbol, node)?;
			captures.push((symbol, slot));
		}
		Ok(captures)
	}

	/// `start do { … }`: a coroutine of its block, given the places of the
	/// locals it uses (copies of them when detached); in a blocking call,
	/// the block runs at once.
	fn start_do(
		&mut self,
		node: NodeId,
		block: NodeId,
		info: &StartInfo,
		immediate: bool,
	) -> Result<Val<'ctx>, Error> {
		let task = self.type_of(node);
		let (ret, throws) = match self.task_layout(task, node) {
			Ok(layout) => layout,
			Err(_) if info.detached => (Repr::Void, false),
			Err(error) => return Err(error),
		};
		let detached = info.detached;
		let eager = |this: &mut Self| -> Result<Option<PointerValue<'ctx>>, Error> {
			let raw = this.inline_task_block(block, ret, throws)?;
			if detached {
				if let Some(raw) = raw {
					this.drop_raw(raw, ret, throws)?;
				}
				return Ok(None);
			}
			Ok(Some(this.finished_block(raw, ret, throws)?))
		};
		let lazy = |this: &mut Self| -> Result<Option<PointerValue<'ctx>>, Error> {
			let captures = this.start_captures(block, node)?;
			let function = this.start_do_function(node, block, &captures, ret, throws, detached)?;
			let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
			for (_, slot) in &captures {
				if detached {
					let value = this.load(*slot)?;
					let value = this.emit_copy(value)?;
					arguments.push(value.value.expect("a value").into());
				} else {
					arguments.push(slot.pointer.into());
				}
			}
			let mode = if immediate { 0 } else { MODE_LAZY };
			let frame = this.call_async(function, &arguments, mode)?;
			if !immediate {
				let flag = this
					.context
					.i32_type()
					.const_int(u64::from(detached), false);
				this.call_runtime("fa_task_spawn", &[frame.into(), flag.into()])?;
			}
			Ok((!detached).then_some(frame))
		};
		let frame = match self.start_kind() {
			Some(true) => lazy(self)?,
			Some(false) => eager(self)?,
			None => self.by_blocking_depth(eager, lazy)?,
		};
		Ok(match frame {
			Some(frame) => self.fresh(Val::new(Repr::Task(task), frame)),
			None => Val::void(),
		})
	}

	/// The block of a `start do` run at once: its raw result (its errors
	/// caught into it).
	fn inline_task_block(
		&mut self,
		block: NodeId,
		ret: Repr,
		throws: bool,
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		let raw_type = self.raw_type(ret, throws);
		let slot = match raw_type {
			Some(ty) => Some(self.alloca_type(ty, "task_result")?),
			None => None,
		};
		let error_slot = self.alloca(Repr::Union, "task_error")?;
		let handler = self.append_block("task_error");
		let after = self.append_block("task_done");
		let mark = self.mark();
		self.frame_mut()
			.catches
			.push((handler, error_slot.pointer, mark));
		let target = ret.has_value().then_some(ret);
		let value = self.region(target, block, |this| this.block(block, ret.has_value()));
		self.frame_mut().catches.pop();
		let value = value?;
		if value.repr != Repr::Never && !self.builder_terminated() {
			let raw = self.raw_of_value(value, ret, throws)?;
			if let (Some(slot), Some(raw)) = (slot, raw) {
				self.builder.build_store(slot, raw)?;
			}
			self.builder.build_unconditional_branch(after)?;
		}
		self.builder.position_at_end(handler);
		if throws {
			let error = self
				.builder
				.build_load(self.pointer_type(), error_slot.pointer, "")?
				.into_pointer_value();
			let raw = self.raw_of_error(error, ret)?;
			if let Some(slot) = slot {
				self.builder.build_store(slot, raw)?;
			}
			self.builder.build_unconditional_branch(after)?;
		} else {
			self.builder.build_unreachable()?;
		}
		self.builder.position_at_end(after);
		Ok(match (slot, raw_type) {
			(Some(slot), Some(ty)) => Some(self.builder.build_load(ty, slot, "")?),
			_ => None,
		})
	}

	fn builder_terminated(&self) -> bool {
		self.builder
			.get_insert_block()
			.is_none_or(|block| block.get_terminator().is_some())
	}

	/// The coroutine of a `start do`: `ptr (captures…, i32 mode)`.
	fn start_do_function(
		&mut self,
		node: NodeId,
		block: NodeId,
		captures: &[(SymbolId, Slot<'ctx>)],
		ret: Repr,
		throws: bool,
		detached: bool,
	) -> Result<FunctionValue<'ctx>, Error> {
		let pointer = self.pointer_type();
		let mut params: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
		for (_, slot) in captures {
			params.push(if detached {
				self.llvm_type(slot.repr).into()
			} else {
				pointer.into()
			});
		}
		params.push(self.context.i32_type().into());
		let function = self.module.add_function(
			"fa.start_do.async",
			pointer.fn_type(&params, false),
			Some(Linkage::Internal),
		);
		function.set_call_conventions(FAST_CALLING_CONVENTION);
		self.runtime.add_attributes(function, &["nounwind"]);
		let name = format!("a `start do` block at {}", self.location(node.index()));
		self.nested_function(function, ret, |this| {
			this.frame_mut().throws = throws;
			let mode = function.get_last_param().expect("a mode").into_int_value();
			this.begin_coroutine(ret, throws, mode, true, false, name)?;
			for (index, (symbol, slot)) in captures.iter().enumerate() {
				let parameter = function.get_nth_param(index as u32).expect("a capture");
				if detached {
					let own = this.alloca(slot.repr, "captured")?;
					this.builder.build_store(own.pointer, parameter)?;
					this.slots.insert(*symbol, own);
					if this.needs_drop(own.repr) {
						this.cleanups.borrow_mut().push(Cleanup::Local(own));
					}
				} else {
					this.slots.insert(
						*symbol,
						Slot {
							pointer: parameter.into_pointer_value(),
							repr: slot.repr,
						},
					);
				}
			}
			this.lazy_suspend(mode, |_| Ok(()))?;
			let value = this.block(block, ret.has_value())?;
			this.build_return(value, block)?;
			this.cleanups.borrow_mut().clear();
			this.end_coroutine()
		})?;
		Ok(function)
	}

	/// Lowers another function from inside the current one (a `start do`,
	/// a combinator), then comes back.
	fn nested_function(
		&mut self,
		function: FunctionValue<'ctx>,
		ret: Repr,
		body: impl FnOnce(&mut Self) -> Result<(), Error>,
	) -> Result<(), Error> {
		let resume = self.builder.get_insert_block().expect("a block");
		let outer = self.frame.take();
		let owner = outer.as_ref().and_then(|frame| frame.owner);
		let saved_slots = std::mem::take(&mut self.slots);
		let saved_owned = std::mem::take(&mut self.owned_locals);
		self.begin_function(function, ret, owner);
		let result = body(self);
		let ended = self.end_function();
		self.frame = outer;
		self.slots = saved_slots;
		self.owned_locals = saved_owned;
		self.builder.position_at_end(resume);
		result?;
		ended
	}

	// ── built-ins ─────────────────────────────────────────────────────────

	/// The built-ins of tasks called as plain calls: `Task.yield()` and
	/// `sleep` blocking, `task.cancel()`, the combinators.
	pub(crate) fn task_builtin(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		match path {
			// in a blocking call, nothing to let run
			"Task.yield" => Ok(Val::void()),
			"std.time.sleep" => {
				let Some(Argument::Given(argument)) = info.arguments.first() else {
					return Err(self.unsupported(node.index(), path));
				};
				let milliseconds = self.expr(*argument)?;
				if milliseconds.repr == Repr::Never {
					return Ok(milliseconds);
				}
				let milliseconds =
					self.convert(milliseconds, Repr::Int(IntRepr::I64), argument.index())?;
				self.call_runtime("fa_sleep", &[milliseconds.int().into()])?;
				Ok(Val::void())
			}
			"Task#cancel" => {
				let View::Call { callee, .. } = self.view(node) else {
					return Err(self.unsupported(node.index(), path));
				};
				let View::Access { target, .. } = self.view(callee) else {
					return Err(self.unsupported(node.index(), path));
				};
				let place = self.place(target)?;
				let frame = self
					.builder
					.build_load(self.pointer_type(), place.pointer, "")?
					.into_pointer_value();
				self.build_task_drop(&self.builder, frame)?;
				let cancelled = self.builder.build_int_to_ptr(
					self.usize.const_int(1, false),
					self.pointer_type(),
					"",
				)?;
				self.builder.build_store(place.pointer, cancelled)?;
				Ok(Val::void())
			}
			"Task.all" | "Task.any" | "Task.settled" => self.combinator(node, path, info),
			_ => Err(self.unsupported(node.index(), path)),
		}
	}

	// ── primitives awaited in an async instance ───────────────────────────

	/// `await sleep(…)`, `await Task.yield()`, `await mutex.lock()`, `await
	/// channel.send(…)`, `await channel.receive()`.
	fn await_primitive(
		&mut self,
		node: NodeId,
		name: &str,
		call: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let coroutine = self.coroutine().expect("a coroutine");
		let handle = coroutine.handle;
		match name {
			"Task.yield" => {
				self.call_runtime("fa_task_yield", &[handle.into()])?;
				self.suspend(Some(node), |_| Ok(()))?;
				Ok(Val::void())
			}
			"sleep" | "std.time.sleep" => {
				let Some(info) = self.analysis.call(call).cloned() else {
					return Err(self.unsupported(node.index(), "this `sleep`"));
				};
				let Some(Argument::Given(argument)) = info.arguments.first() else {
					return Err(self.unsupported(node.index(), "this `sleep`"));
				};
				let milliseconds = self.expr(*argument)?;
				if milliseconds.repr == Repr::Never {
					return Ok(milliseconds);
				}
				let milliseconds =
					self.convert(milliseconds, Repr::Int(IntRepr::I64), argument.index())?;
				let wait = self.words(WAIT_WORDS, "wait")?;
				self.call_runtime(
					"fa_sleep_async",
					&[wait.into(), handle.into(), milliseconds.int().into()],
				)?;
				self.suspend(Some(node), |this| {
					this.call_runtime("fa_timer_cancel", &[wait.into()])
				})?;
				Ok(Val::void())
			}
			"Mutex.lock" => self.await_lock(node, call),
			"Channel.send" => self.await_send(node, call),
			"Channel.receive" => self.await_receive(node, call),
			// waits for room in the pool's queue
			"WorkerPool.submit" => {
				let Some(info) = self.analysis.call(call).cloned() else {
					return Err(self.unsupported(node.index(), "this `submit`"));
				};
				self.pool_submit(call, &info, true)
			}
			// std.fs, std.net (io.rs)
			"File.readText" | "File.read" | "File.writeText" | "TcpListener.accept"
			| "TcpStream.connect" | "TcpStream.read" | "TcpStream.write" => self.await_io(node, name, call),
			// a call on a worker's thread
			"worker" => {
				self.tasks.call_modes.insert(call, CallMode::Await);
				let value = self.expr(call);
				self.tasks.call_modes.remove(&call);
				value
			}
			_ => self.expr(call),
		}
	}

	/// Branches on the result of an `fa_*_async`: `ready` at once, the
	/// suspension otherwise (whose `on_destroy` cancels the wait), then the
	/// status of the wait: `done` when handed over, `closed` otherwise.
	fn await_wait(
		&mut self,
		node: NodeId,
		result: IntValue<'ctx>,
		wait: PointerValue<'ctx>,
		on_destroy: impl FnOnce(&mut Self) -> Result<(), Error>,
	) -> Result<(BasicBlock<'ctx>, BasicBlock<'ctx>), Error> {
		let i32 = self.context.i32_type();
		let done = self.append_block("wait_done");
		let closed = self.append_block("wait_closed");
		let waiting = self.append_block("waiting");
		self.builder.build_switch(
			result,
			waiting,
			&[
				(i32.const_int(AWAIT_READY, false), done),
				(i32.const_int(AWAIT_CLOSED, false), closed),
			],
		)?;
		self.builder.position_at_end(waiting);
		self.suspend(Some(node), on_destroy)?;
		let status = self
			.runtime_value("fa_wait_status", &[wait.into()])?
			.into_int_value();
		let handed = self.builder.build_int_compare(
			IntPredicate::EQ,
			status,
			i32.const_int(WAIT_DONE, false),
			"",
		)?;
		self.builder
			.build_conditional_branch(handed, done, closed)?;
		Ok((done, closed))
	}

	/// The handle of a method call's receiver (a mutex, a channel).
	fn sync_receiver(
		&mut self,
		call: NodeId,
	) -> Result<Option<(PointerValue<'ctx>, TypeId)>, Error> {
		let receiver = self.receiver(call)?;
		if receiver.repr == Repr::Never {
			return Ok(None);
		}
		let Repr::Sync(ty) = receiver.repr else {
			return Err(self.unsupported(call.index(), "this receiver"));
		};
		Ok(Some((
			receiver.value.expect("a handle").into_pointer_value(),
			ty,
		)))
	}

	/// `await mutex.lock()`: the guard, the lock handed over in line.
	fn await_lock(&mut self, node: NodeId, call: NodeId) -> Result<Val<'ctx>, Error> {
		let Some((block, _)) = self.sync_receiver(call)? else {
			return Ok(Val::never());
		};
		let handle = self.coroutine().expect("a coroutine").handle;
		let wait = self.words(WAIT_WORDS, "wait")?;
		let site = format!("Mutex.lock at {}", self.location(call.index()));
		let site = self.c_string(&site);
		let result = self
			.runtime_value(
				"fa_mutex_lock_async",
				&[block.into(), wait.into(), handle.into(), site.into()],
			)?
			.into_int_value();
		let (done, closed) = self.await_wait(node, result, wait, |this| {
			this.call_runtime("fa_mutex_cancel", &[block.into(), wait.into()])
		})?;
		self.builder.position_at_end(closed);
		self.builder.build_unreachable()?;
		self.builder.position_at_end(done);
		let guard_type = self.type_of(call);
		Ok(self.fresh(Val::new(Repr::Sync(guard_type), block)))
	}

	/// `await channel.send(value)`: sent at once, or handed over in line;
	/// `ChannelClosed` when the channel is closed (the value dropped).
	fn await_send(&mut self, node: NodeId, call: NodeId) -> Result<Val<'ctx>, Error> {
		let Some((block, channel_type)) = self.sync_receiver(call)? else {
			return Ok(Val::never());
		};
		let Some((_, item)) = self.sync_kind(channel_type) else {
			return Err(self.unsupported(call.index(), "this channel"));
		};
		let item_repr = self.item_repr(item, call)?;
		let Some(info) = self.analysis.call(call).cloned() else {
			return Err(self.unsupported(call.index(), "this `send`"));
		};
		let Some(argument) = info.arguments.iter().find_map(|argument| match argument {
			Argument::Given(value) => Some(*value),
			_ => None,
		}) else {
			return Err(self.unsupported(call.index(), "this `send`"));
		};
		let value = self.expr(argument)?;
		if value.repr == Repr::Never {
			return Ok(value);
		}
		// the channel owns what it receives
		let value = self.owned_as(value, item_repr, Some(argument))?;
		let slot = self.alloca(item_repr, "sent")?;
		if let Some(raw) = value.value {
			self.builder.build_store(slot.pointer, raw)?;
		}
		let handle = self.coroutine().expect("a coroutine").handle;
		let wait = self.words(WAIT_WORDS, "wait")?;
		let site = format!("Channel.send at {}", self.location(call.index()));
		let site = self.c_string(&site);
		let result = self
			.runtime_value(
				"fa_channel_send_async",
				&[
					block.into(),
					slot.pointer.into(),
					wait.into(),
					handle.into(),
					site.into(),
				],
			)?
			.into_int_value();
		let i32 = self.context.i32_type();
		let (done, closed) = self.await_wait(node, result, wait, |this| {
			let status = this
				.runtime_value("fa_channel_cancel", &[block.into(), wait.into()])?
				.into_int_value();
			let sent = this.builder.build_int_compare(
				IntPredicate::EQ,
				status,
				i32.const_int(WAIT_DONE, false),
				"",
			)?;
			let kept = this.append_block("send_cancelled");
			let after = this.append_block("send_cancel_done");
			this.builder.build_conditional_branch(sent, after, kept)?;
			this.builder.position_at_end(kept);
			let left = this.load(slot)?;
			this.emit_drop(left)?;
			this.builder.build_unconditional_branch(after)?;
			this.builder.position_at_end(after);
			Ok(())
		})?;
		self.builder.position_at_end(closed);
		let left = self.load(slot)?;
		self.emit_drop(left)?;
		let callee_type = match self.view(call) {
			View::Call { callee, .. } => self.type_of(callee),
			_ => TypeId::UNKNOWN,
		};
		match self.channel_closed_error(callee_type, call)? {
			Some(error) => self.raise(error)?,
			None => {
				self.builder.build_unconditional_branch(done)?;
			}
		}
		self.builder.position_at_end(done);
		Ok(Val::void())
	}

	/// `await channel.receive()`: the next value, or `none` once the
	/// channel is closed and empty.
	fn await_receive(&mut self, node: NodeId, call: NodeId) -> Result<Val<'ctx>, Error> {
		let Some((block, channel_type)) = self.sync_receiver(call)? else {
			return Ok(Val::never());
		};
		let Some((_, item)) = self.sync_kind(channel_type) else {
			return Err(self.unsupported(call.index(), "this channel"));
		};
		let item_repr = self.item_repr(item, call)?;
		let repr = self.node_repr(call)?;
		let Repr::Optional(scalar) = repr else {
			return Err(self.unsupported(call.index(), "this `receive`"));
		};
		let slot = self.alloca(item_repr, "received")?;
		let handle = self.coroutine().expect("a coroutine").handle;
		let wait = self.words(WAIT_WORDS, "wait")?;
		let site = format!("Channel.receive at {}", self.location(call.index()));
		let site = self.c_string(&site);
		let result = self
			.runtime_value(
				"fa_channel_receive_async",
				&[
					block.into(),
					slot.pointer.into(),
					wait.into(),
					handle.into(),
					site.into(),
				],
			)?
			.into_int_value();
		let i32 = self.context.i32_type();
		let (done, closed) = self.await_wait(node, result, wait, |this| {
			let status = this
				.runtime_value("fa_channel_cancel", &[block.into(), wait.into()])?
				.into_int_value();
			let got = this.builder.build_int_compare(
				IntPredicate::EQ,
				status,
				i32.const_int(WAIT_DONE, false),
				"",
			)?;
			let drop = this.append_block("receive_cancelled");
			let after = this.append_block("receive_cancel_done");
			this.builder.build_conditional_branch(got, drop, after)?;
			this.builder.position_at_end(drop);
			let received = this.load(slot)?;
			this.emit_drop(received)?;
			this.builder.build_unconditional_branch(after)?;
			this.builder.position_at_end(after);
			Ok(())
		})?;
		let join = self.append_block("received_or_not");
		self.builder.position_at_end(done);
		let loaded = self.load(slot)?;
		let value = self.convert(loaded, scalar.repr(), call.index())?;
		let ty = self.llvm_type(repr);
		let present = self.context.bool_type().const_int(1, false);
		let pair = self.pack_optional(
			&self.builder,
			scalar,
			present,
			value.value.expect("a value"),
		)?;
		let done_end = self.builder.get_insert_block().expect("a block");
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(closed);
		self.builder.build_unconditional_branch(join)?;
		self.builder.position_at_end(join);
		let phi = self.builder.build_phi(ty, "")?;
		let none = super::zero_of(ty);
		phi.add_incoming(&[(&pair, done_end), (&none, closed)]);
		Ok(self.fresh(Val::new(repr, phi.as_basic_value())))
	}

	// ── the coop budget (A2) ──────────────────────────────────────────────

	/// At each iteration of a loop of an async instance: spends a unit of
	/// the coop budget; when spent and the turn lasted more than 2 ms, the
	/// task yields.
	pub(crate) fn coop_point(&mut self) -> Result<(), Error> {
		let Some(coroutine) = self.coroutine() else {
			return Ok(());
		};
		if !coroutine.coop || !self.tasks.asynchronous {
			return Ok(());
		}
		let i32 = self.context.i32_type();
		let budget = self.runtime_global("fa_coop_budget")?;
		let value = self
			.builder
			.build_load(i32, budget, "budget")?
			.into_int_value();
		let value = self
			.builder
			.build_int_sub(value, i32.const_int(1, false), "")?;
		self.builder.build_store(budget, value)?;
		let spent =
			self.builder
				.build_int_compare(IntPredicate::SLE, value, i32.const_zero(), "spent")?;
		let check = self.append_block("budget_spent");
		let yields = self.append_block("coop_yield");
		let go_on = self.append_block("coop_go_on");
		let branch = self.builder.build_conditional_branch(spent, check, go_on)?;
		let weights = self.context.metadata_node(&[
			self.context.metadata_string("branch_weights").into(),
			i32.const_int(1, false).into(),
			i32.const_int(127, false).into(),
		]);
		branch
			.set_metadata(weights, self.context.get_kind_id("prof"))
			.map_err(|error| Error::Llvm(error.to_string()))?;
		self.builder.position_at_end(check);
		let must = self
			.runtime_value("fa_coop_back_edge", &[])?
			.into_int_value();
		let must = self
			.builder
			.build_int_compare(IntPredicate::NE, must, i32.const_zero(), "")?;
		self.builder.build_conditional_branch(must, yields, go_on)?;
		self.builder.position_at_end(yields);
		self.call_runtime("fa_task_yield", &[coroutine.handle.into()])?;
		self.suspend(None, |_| Ok(()))?;
		self.builder.build_unconditional_branch(go_on)?;
		self.builder.position_at_end(go_on);
		Ok(())
	}

	// ── Task.all, Task.any, Task.settled ──────────────────────────────────

	/// `Task.all(tasks)`, `Task.any(tasks)`, `Task.settled(tasks)`: a task
	/// joining the tasks (moved in). In a blocking call, they are finished
	/// blocks, joined at once.
	fn combinator(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let Some(Argument::Given(argument)) = info.arguments.first().cloned() else {
			return Err(self.unsupported(node.index(), path));
		};
		let immediate = self.tasks.immediate == Some(node);
		self.tasks.immediate = None;
		let tasks = self.expr(argument)?;
		if tasks.repr == Repr::Never {
			return Ok(tasks);
		}
		let Repr::Array(item) = tasks.repr else {
			return Err(self.unsupported(node.index(), "these tasks"));
		};
		let tasks = self.take(tasks, Some(argument))?;
		let array = tasks.value.expect("an array").into_pointer_value();
		let task = self.type_of(node);
		let (ret, throws) = self.task_layout(task, node)?;
		let kind = path.trim_start_matches("Task.");
		let lazy =
			self.tasks.asynchronous && !matches!(self.frame().task, FrameTask::Blocking { .. });
		let frame = if lazy {
			let function = self.join_function(node, kind, item, task, ret, throws)?;
			let mode = if immediate { 0 } else { MODE_LAZY };
			let frame = self.call_async(function, &[array.into()], mode)?;
			if !immediate {
				let flag = self.context.i32_type().const_zero();
				self.call_runtime("fa_task_spawn", &[frame.into(), flag.into()])?;
			}
			frame
		} else {
			let raw = self.join_body(node, kind, array, item, ret, throws, None)?;
			self.finished_block(raw, ret, throws)?
		};
		Ok(self.fresh(Val::new(Repr::Task(task), frame)))
	}

	/// The coroutine of a combinator: `ptr (ptr tasks, i32 mode)`.
	fn join_function(
		&mut self,
		node: NodeId,
		kind: &str,
		item: TypeId,
		task: TypeId,
		ret: Repr,
		throws: bool,
	) -> Result<FunctionValue<'ctx>, Error> {
		let generic = !self.bindings.borrow().is_empty();
		let key = format!("{kind}.{item:?}.{task:?}.{}", self.file.0);
		if !generic && let Some(function) = self.tasks.joins.get(&key) {
			return Ok(*function);
		}
		let pointer = self.pointer_type();
		let function = self.module.add_function(
			&format!("fa.task_{kind}.async"),
			pointer.fn_type(&[pointer.into(), self.context.i32_type().into()], false),
			Some(Linkage::Internal),
		);
		function.set_call_conventions(FAST_CALLING_CONVENTION);
		self.runtime.add_attributes(function, &["nounwind"]);
		if !generic {
			self.tasks.joins.insert(key, function);
		}
		let kind = kind.to_string();
		self.nested_function(function, ret, |this| {
			this.frame_mut().throws = throws;
			let array = function
				.get_nth_param(0)
				.expect("the tasks")
				.into_pointer_value();
			let mode = function.get_nth_param(1).expect("a mode").into_int_value();
			this.begin_coroutine(ret, throws, mode, false, false, format!("Task.{kind}"))?;
			let tasks = Val::new(Repr::Array(item), array);
			this.lazy_suspend(mode, |this| this.emit_drop(tasks))?;
			let handle = this.coroutine().expect("a coroutine").handle;
			let raw = this.join_body(node, &kind, array, item, ret, throws, Some(handle))?;
			this.return_raw(raw)?;
			this.end_coroutine()
		})?;
		Ok(function)
	}

	/// Joins the tasks of an array (owned, dropped at the end): in a
	/// coroutine (`frame`), waits for them; else they must be done. The raw
	/// result of the combinator.
	#[allow(clippy::too_many_arguments)]
	fn join_body(
		&mut self,
		node: NodeId,
		kind: &str,
		array: PointerValue<'ctx>,
		item: TypeId,
		ret: Repr,
		throws: bool,
		frame: Option<PointerValue<'ctx>>,
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		let i64 = self.context.i64_type();
		let pointer = self.pointer_type();
		let tasks = Val::new(Repr::Array(item), array);
		let (task_ret, task_throws) = self.task_layout(item, node)?;
		let join = self.words(JOIN_WORDS, "join")?;
		self.call_runtime("fa_join_init", &[join.into()])?;
		let (items, length) = self.array_parts(array)?;
		let task_repr = Repr::Task(item);
		// every task joins
		let index = self.alloca(Repr::Int(IntRepr::I64), "index")?;
		self.builder.build_store(index.pointer, i64.const_zero())?;
		let add_test = self.append_block("join_add");
		let add_body = self.append_block("join_add_body");
		let added = self.append_block("join_added");
		self.builder.build_unconditional_branch(add_test)?;
		self.builder.position_at_end(add_test);
		let position = self.load(index)?.int();
		let more = self
			.builder
			.build_int_compare(IntPredicate::SLT, position, length, "")?;
		self.builder
			.build_conditional_branch(more, add_body, added)?;
		self.builder.position_at_end(add_body);
		let at = self.item_pointer(items, task_repr, position)?;
		let member = self.builder.build_load(pointer, at, "")?;
		self.call_runtime("fa_join_add", &[join.into(), member.into()])?;
		let next = self
			.builder
			.build_int_add(position, i64.const_int(1, false), "")?;
		self.builder.build_store(index.pointer, next)?;
		self.builder.build_unconditional_branch(add_test)?;
		self.builder.position_at_end(added);

		let raw_type = self.raw_type(ret, throws);
		let result = match raw_type {
			Some(ty) => Some(self.alloca_type(ty, "joined")?),
			None => None,
		};
		let wait = self.append_block("join_next");
		let finished = self.append_block("join_finished");
		let complete = self.append_block("join_complete");
		let done = self.append_block("join_done");
		self.builder.build_unconditional_branch(wait)?;
		self.builder.position_at_end(wait);
		let self_frame = frame.unwrap_or_else(|| pointer.const_null());
		let ended = self
			.runtime_value("fa_join_next", &[join.into(), self_frame.into()])?
			.into_pointer_value();
		let address = self.builder.build_ptr_to_int(ended, self.usize, "")?;
		let none = self.builder.build_int_compare(
			IntPredicate::EQ,
			address,
			self.usize.const_zero(),
			"",
		)?;
		let all = self.builder.build_int_compare(
			IntPredicate::EQ,
			address,
			self.usize.const_int(1, false),
			"",
		)?;
		let suspend = self.append_block("join_wait");
		let one = self.append_block("join_one");
		self.builder.build_conditional_branch(none, suspend, one)?;
		self.builder.position_at_end(suspend);
		if frame.is_some() {
			self.suspend(Some(node), |this| this.emit_drop(tasks))?;
			self.builder.build_unconditional_branch(wait)?;
		} else {
			self.call_noreturn("fa_task_stuck", &[])?;
		}
		self.builder.position_at_end(one);
		self.builder
			.build_conditional_branch(all, complete, finished)?;

		// a task ended
		self.builder.position_at_end(finished);
		let promise_type = self.promise_type(task_ret, task_throws);
		match kind {
			"any" => {
				let raw = self.take_raw(ended, task_ret, task_throws, promise_type)?;
				self.emit_drop(tasks)?;
				if let (Some(slot), Some(raw)) = (result, raw) {
					self.builder.build_store(slot, raw)?;
				}
				self.builder.build_unconditional_branch(done)?;
			}
			"all" if task_throws => {
				let promise = self.promise_of(ended)?;
				let result_field = self
					.builder
					.build_struct_gep(promise_type, promise, 1, "")?;
				let error = if task_ret.has_value() {
					let raw_type = self.raw_type(task_ret, true).expect("a pair");
					let pair = self
						.builder
						.build_load(raw_type, result_field, "")?
						.into_struct_value();
					self.builder
						.build_extract_value(pair, 1, "")?
						.into_pointer_value()
				} else {
					self.builder
						.build_load(pointer, result_field, "")?
						.into_pointer_value()
				};
				let failed = self.builder.build_is_not_null(error, "")?;
				let fail = self.append_block("join_failed");
				self.builder.build_conditional_branch(failed, fail, wait)?;
				self.builder.position_at_end(fail);
				// the error is taken, the other tasks cancelled
				self.clear_result_flag(promise)?;
				self.emit_drop(tasks)?;
				let raw = self.raw_of_error(error, ret)?;
				if let Some(slot) = result {
					self.builder.build_store(slot, raw)?;
				}
				self.builder.build_unconditional_branch(done)?;
			}
			_ => {
				self.builder.build_unconditional_branch(wait)?;
			}
		}

		// every task ended
		self.builder.position_at_end(complete);
		match kind {
			"any" => {
				self.emit_drop(tasks)?;
				self.call_noreturn("fa_task_stuck", &[])?;
			}
			_ => {
				let Repr::Array(result_item) = ret else {
					return Err(self.unsupported(node.index(), "this combinator"));
				};
				let result_item_repr = self.item_repr(result_item, node)?;
				let results = self.new_array(result_item_repr, length, length)?;
				self.builder.build_store(index.pointer, i64.const_zero())?;
				let test = self.append_block("join_collect");
				let body = self.append_block("join_collect_body");
				let collected = self.append_block("join_collected");
				self.builder.build_unconditional_branch(test)?;
				self.builder.position_at_end(test);
				let position = self.load(index)?.int();
				let more =
					self.builder
						.build_int_compare(IntPredicate::SLT, position, length, "")?;
				self.builder
					.build_conditional_branch(more, body, collected)?;
				self.builder.position_at_end(body);
				let at = self.item_pointer(items, task_repr, position)?;
				let member = self
					.builder
					.build_load(pointer, at, "")?
					.into_pointer_value();
				let raw = self.take_raw(member, task_ret, task_throws, promise_type)?;
				let (items_out, _) = self.array_parts(results)?;
				let out = self.item_pointer(items_out, result_item_repr, position)?;
				if kind == "settled" {
					self.store_settled(raw, task_ret, task_throws, result_item, out, node)?;
				} else if let Some(raw) = raw {
					let value = if task_throws && task_ret.has_value() {
						self.builder
							.build_extract_value(raw.into_struct_value(), 0, "")?
					} else {
						raw
					};
					if task_ret.has_value() {
						self.builder.build_store(out, value)?;
					}
				}
				let next = self
					.builder
					.build_int_add(position, i64.const_int(1, false), "")?;
				self.builder.build_store(index.pointer, next)?;
				self.builder.build_unconditional_branch(test)?;
				self.builder.position_at_end(collected);
				self.emit_drop(tasks)?;
				let value = Val::new(ret, results);
				let raw = self.raw_of_value(value, ret, throws)?;
				if let (Some(slot), Some(raw)) = (result, raw) {
					self.builder.build_store(slot, raw)?;
				}
				self.builder.build_unconditional_branch(done)?;
			}
		}
		self.builder.position_at_end(done);
		Ok(match (result, raw_type) {
			(Some(slot), Some(ty)) => Some(self.builder.build_load(ty, slot, "")?),
			_ => None,
		})
	}

	/// Moves the raw result out of a done task (it stays, to be dropped).
	fn take_raw(
		&mut self,
		frame: PointerValue<'ctx>,
		ret: Repr,
		throws: bool,
		promise_type: StructType<'ctx>,
	) -> Result<Option<BasicValueEnum<'ctx>>, Error> {
		let _ = promise_type;
		let promise = self.promise_of(frame)?;
		let raw = self.load_promise_raw(promise, ret, throws)?;
		self.clear_result_flag(promise)?;
		Ok(raw)
	}

	/// An outcome of `Task.settled`: the value or the error, as a `T | E`.
	fn store_settled(
		&mut self,
		raw: Option<BasicValueEnum<'ctx>>,
		ret: Repr,
		throws: bool,
		item: TypeId,
		out: PointerValue<'ctx>,
		node: NodeId,
	) -> Result<(), Error> {
		let item_repr = self.item_repr(item, node)?;
		let value_of = |this: &mut Self,
		                value: Option<BasicValueEnum<'ctx>>|
		 -> Result<Option<BasicValueEnum<'ctx>>, Error> {
			// moved out of the task (`take_raw`): owned, no copy
			let value = match value {
				Some(value) => this.fresh(Val::new(ret, value)),
				None => return Ok(None),
			};
			let converted = if item_repr == Repr::Union && value.repr != Repr::Union {
				this.inject(value, item, node)?
			} else {
				this.convert(value, item_repr, node.index())?
			};
			// the array owns it
			let converted = this.take(converted, None)?;
			Ok(converted.value)
		};
		if !throws {
			if let Some(value) = value_of(self, raw)? {
				self.builder.build_store(out, value)?;
			}
			return Ok(());
		}
		let raw = raw.expect("a result");
		let (value, error) = if ret.has_value() {
			let pair = raw.into_struct_value();
			(
				Some(self.builder.build_extract_value(pair, 0, "")?),
				self.builder
					.build_extract_value(pair, 1, "")?
					.into_pointer_value(),
			)
		} else {
			(None, raw.into_pointer_value())
		};
		let failed = self.builder.build_is_not_null(error, "")?;
		let fail = self.append_block("settled_error");
		let succeed = self.append_block("settled_value");
		let after = self.append_block("settled");
		self.builder
			.build_conditional_branch(failed, fail, succeed)?;
		self.builder.position_at_end(fail);
		self.builder.build_store(out, error)?;
		self.builder.build_unconditional_branch(after)?;
		self.builder.position_at_end(succeed);
		if let Some(value) = value_of(self, value)? {
			self.builder.build_store(out, value)?;
		}
		self.builder.build_unconditional_branch(after)?;
		self.builder.position_at_end(after);
		Ok(())
	}
}

// ── workers (N3: design/async.md §3.4, A9, A10) ──────────────────────────

/// Where a worker's state is in its block (`WORKER_STATE` of the runtime).
const WORKER_STATE: u64 = 256;

impl<'ctx> Lowering<'_, 'ctx> {
	/// Whether a type is a worker's handle in an asynchronous program (a
	/// block of the runtime: `Repr::Sync`, its thread, its state).
	fn is_threaded_worker(&self, ty: TypeId) -> bool {
		self.tasks.asynchronous
			&& matches!(
				self.types().kind(ty),
				TypeKind::Sync(fa_analyzer::types::SyncKind::Worker, _)
			)
	}

	/// `MathWorker { … }` in an asynchronous program: the state is built,
	/// moved into a new worker's block, and its thread started. `None` for
	/// another literal.
	pub(crate) fn worker_literal(&mut self, node: NodeId) -> Result<Option<Val<'ctx>>, Error> {
		let ty = self.type_of(node);
		if !self.is_threaded_worker(ty) || self.tasks.worker_state == Some(node) {
			return Ok(None);
		}
		let TypeKind::Sync(_, state_type) = self.types().kind(ty) else {
			return Ok(None);
		};
		let previous = self.tasks.worker_state.replace(node);
		let state = self.object_literal(node);
		self.tasks.worker_state = previous;
		let state = state?;
		if state.repr == Repr::Never {
			return Ok(Some(state));
		}
		let repr = self.repr_of(state_type, node.index())?;
		let state = self.owned_as(state, repr, Some(node))?;
		let size = self.size_of_repr(repr)?;
		let drop = self.drop_at_function(repr, WORKER_STATE)?;
		let block = self
			.runtime_value("fa_worker_new", &[size.into(), drop.into()])?
			.into_pointer_value();
		let at = self.byte_offset(block, WORKER_STATE)?;
		if let Some(raw) = state.value {
			self.builder.build_store(at, raw)?;
		}
		self.call_runtime("fa_worker_start", &[block.into()])?;
		Ok(Some(self.fresh(Val::new(Repr::Sync(ty), block))))
	}

	/// `worker.stop()`: in an asynchronous program, the worker's thread
	/// ends (its handle is consumed; its block goes with its scope).
	pub(crate) fn stop_worker(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		if !self.tasks.asynchronous {
			return Ok(Val::void());
		}
		let worker = self.receiver(node)?;
		if worker.repr == Repr::Never {
			return Ok(worker);
		}
		let Repr::Sync(_) = worker.repr else {
			return Ok(Val::void());
		};
		self.call_runtime("fa_worker_stop", &[worker.value.expect("a worker").into()])?;
		Ok(Val::void())
	}

	/// A call of a method of a worker, in an asynchronous program: a message
	/// to its thread (the arguments copied), whose frame is the task of the
	/// call. Awaited, the caller waits for it; started, it is the task; a
	/// blocking call waits on the worker. `None` for another call.
	pub(crate) fn worker_call(
		&mut self,
		node: NodeId,
		function: &Function<'ctx>,
		info: &CallInfo,
	) -> Result<Option<Val<'ctx>>, Error> {
		if !self.tasks.asynchronous
			|| !matches!(info.arguments.first(), Some(Argument::Receiver(_)))
		{
			return Ok(None);
		}
		let View::Call { callee, .. } = self.view(node) else {
			return Ok(None);
		};
		let View::Access { target, .. } = self.view(self.ungrouped(callee)) else {
			return Ok(None);
		};
		if !self.is_threaded_worker(self.type_of(target)) {
			return Ok(None);
		}
		if function.lambda {
			return Err(self.unsupported(node.index(), "this worker method"));
		}
		let mode = self.tasks.call_modes.remove(&node);
		let worker = self.expr(target)?;
		if worker.repr == Repr::Never {
			return Ok(Some(worker));
		}
		let worker = worker.value.expect("a worker").into_pointer_value();
		// the arguments, copied (the worker's thread owns them)
		let mut fields: Vec<(Option<Repr>, BasicValueEnum<'ctx>)> = Vec::new();
		let mut given: Vec<(usize, NodeId)> = info
			.arguments
			.iter()
			.enumerate()
			.filter_map(|(position, argument)| match argument {
				Argument::Given(value) => Some((position, *value)),
				_ => None,
			})
			.collect();
		given.sort_by_key(|(_, value)| self.tree.spans[value.index()].start);
		let mut evaluated: FxHashMap<usize, Val<'ctx>> = FxHashMap::default();
		for (position, value) in given {
			if function.by_pointer.get(position) == Some(&true) {
				return Err(self.unsupported(value.index(), "a mutated argument of a worker"));
			}
			let lowered = self.expr(value)?;
			if lowered.repr == Repr::Never {
				return Ok(Some(lowered));
			}
			evaluated.insert(position, lowered);
		}
		for (position, ((repr, has_default), argument)) in
			function.params.iter().zip(&info.arguments).enumerate()
		{
			match argument {
				Argument::Receiver(_) => {}
				Argument::Given(value) => {
					let lowered = self.convert(evaluated[&position], *repr, value.index())?;
					let copy = self.emit_copy(lowered)?;
					let raw = copy
						.value
						.unwrap_or_else(|| super::zero_of(self.llvm_type(*repr)));
					fields.push((Some(*repr), raw));
					if *has_default {
						fields.push((None, self.context.bool_type().const_int(1, false).into()));
					}
				}
				Argument::Default if *has_default => {
					fields.push((None, super::zero_of(self.llvm_type(*repr))));
					fields.push((None, self.context.bool_type().const_zero().into()));
				}
				_ => return Err(self.unsupported(node.index(), "these arguments of a worker")),
			}
		}
		let types: Vec<BasicTypeEnum<'ctx>> =
			fields.iter().map(|(_, raw)| raw.get_type()).collect();
		let block_type = self.context.struct_type(&types, false);
		let size = block_type.size_of().expect("a sized block");
		let size = self.builder.build_int_cast(size, self.usize, "")?;
		let size = self
			.builder
			.build_int_add(size, self.usize.const_int(1, false), "")?;
		let arguments = self.allocate(size)?;
		for (index, (_, raw)) in fields.iter().enumerate() {
			let at = self
				.builder
				.build_struct_gep(block_type, arguments, index as u32, "")?;
			self.builder.build_store(at, *raw)?;
		}
		let reprs: Vec<Option<Repr>> = fields.iter().map(|(repr, _)| *repr).collect();
		let drop_arguments = self.worker_drop_arguments(function, block_type, &reprs)?;
		let run = self.worker_run(function, block_type, drop_arguments)?;
		// the frame: a task's layout, its resume and destroy functions the runtime's
		let (ret, throws) = (function.ret, function.throws);
		let promise_type = self.promise_type(ret, throws);
		let frame_size = self.finished_size(promise_type);
		let frame_size = self
			.builder
			.build_int_cast(frame_size, self.context.i64_type(), "")?;
		let frame = self
			.runtime_value("fa_call_new", &[frame_size.into()])?
			.into_pointer_value();
		let (resume, destroy) = self.worker_frame_functions()?;
		let pointer = self.pointer_type();
		self.builder
			.build_store(frame, resume.as_global_value().as_pointer_value())?;
		let destroy_slot = unsafe {
			self.builder.build_in_bounds_gep(
				pointer,
				frame,
				&[self.usize.const_int(1, false)],
				"",
			)?
		};
		self.builder
			.build_store(destroy_slot, destroy.as_global_value().as_pointer_value())?;
		let dispose = self.worker_dispose(ret, throws)?;
		let coroutine = self.in_coroutine();
		let call_mode: u64 = match mode {
			Some(CallMode::Await) if coroutine => 1,
			Some(CallMode::Start { detached, .. }) if self.start_kind() == Some(true) => {
				if detached { 2 } else { 1 }
			}
			_ => 0,
		};
		let i32 = self.context.i32_type();
		self.call_runtime(
			"fa_worker_call",
			&[
				worker.into(),
				frame.into(),
				run.as_global_value().as_pointer_value().into(),
				arguments.into(),
				drop_arguments.as_global_value().as_pointer_value().into(),
				dispose.as_global_value().as_pointer_value().into(),
				i32.const_int(call_mode, false).into(),
			],
		)?;
		match mode {
			Some(CallMode::Start { detached, task, .. }) => {
				if detached {
					// done already when run at once (a blocking start)
					if call_mode == 0 {
						self.build_frame_call(&self.builder, frame, 1)?;
					}
					return Ok(Some(Val::void()));
				}
				Ok(Some(self.fresh(Val::new(Repr::Task(task), frame))))
			}
			_ => {
				if call_mode == 1 {
					self.await_frame(frame, Some(node))?;
				}
				let raw = self.take_result(frame, ret, throws)?;
				Ok(Some(self.raw_to_value(raw, ret, throws)?))
			}
		}
	}

	/// The resume and destroy functions of the frames of worker calls
	/// (`fastcc`, as `llvm.coro.resume` and `llvm.coro.destroy` call them):
	/// they call `fa_call_resume` and `fa_call_destroy`.
	fn worker_frame_functions(&self) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		let make = |name: &str, target: &str| -> Result<FunctionValue<'ctx>, Error> {
			if let Some(function) = self.module.get_function(name) {
				return Ok(function);
			}
			let pointer = self.pointer_type();
			let function = self.module.add_function(
				name,
				self.context.void_type().fn_type(&[pointer.into()], false),
				Some(Linkage::Internal),
			);
			function.set_call_conventions(FAST_CALLING_CONVENTION);
			self.runtime.add_attributes(function, &["nounwind"]);
			let builder = self.context.create_builder();
			builder.position_at_end(self.context.append_basic_block(function, "entry"));
			let frame = function.get_nth_param(0).expect("a frame");
			builder.build_call(self.runtime.function(target)?, &[frame.into()], "")?;
			builder.build_return(None)?;
			Ok(function)
		};
		Ok((
			make("fa.worker_call.resume", "fa_call_resume")?,
			make("fa.worker_call.destroy", "fa_call_destroy")?,
		))
	}

	/// `fa.worker.drop_arguments(ptr arguments)`: drops the copied arguments
	/// of a call of `function`, then frees their block.
	fn worker_drop_arguments(
		&self,
		function: &Function<'ctx>,
		block_type: StructType<'ctx>,
		reprs: &[Option<Repr>],
	) -> Result<FunctionValue<'ctx>, Error> {
		let name = format!(
			"fa.worker.drop_arguments.{}",
			function.value.get_name().to_string_lossy()
		);
		if let Some(existing) = self.module.get_function(&name) {
			return Ok(existing);
		}
		let pointer = self.pointer_type();
		let drop = self.module.add_function(
			&name,
			self.context.void_type().fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(drop, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(drop, "entry"));
		let arguments = drop
			.get_nth_param(0)
			.expect("arguments")
			.into_pointer_value();
		for (index, repr) in reprs.iter().enumerate() {
			let Some(repr) = repr else { continue };
			if !self.needs_drop(*repr) {
				continue;
			}
			let at = builder.build_struct_gep(block_type, arguments, index as u32, "")?;
			let value = builder.build_load(self.llvm_type(*repr), at, "")?;
			self.call_drop(&builder, *repr, value)?;
		}
		let size = block_type.size_of().expect("a sized block");
		let size = builder.build_int_cast(size, self.usize, "")?;
		let size = builder.build_int_add(size, self.usize.const_int(1, false), "")?;
		builder.build_call(
			self.runtime.free()?,
			&[
				arguments.into(),
				size.into(),
				self.usize.const_int(16, false).into(),
			],
			"",
		)?;
		builder.build_return(None)?;
		Ok(drop)
	}

	/// `fa.worker.run(ptr state, ptr arguments, ptr frame)`: calls the
	/// synchronous instance of `function` on the worker's thread, the state
	/// as its receiver, stores its result in the frame's promise, then drops
	/// the arguments.
	fn worker_run(
		&mut self,
		function: &Function<'ctx>,
		block_type: StructType<'ctx>,
		drop_arguments: FunctionValue<'ctx>,
	) -> Result<FunctionValue<'ctx>, Error> {
		let name = format!(
			"fa.worker.run.{}",
			function.value.get_name().to_string_lossy()
		);
		if let Some(existing) = self.module.get_function(&name) {
			return Ok(existing);
		}
		let pointer = self.pointer_type();
		let run = self.module.add_function(
			&name,
			self.context
				.void_type()
				.fn_type(&[pointer.into(), pointer.into(), pointer.into()], false),
			Some(Linkage::Internal),
		);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(run, "entry"));
		let state = run.get_nth_param(0).expect("a state").into_pointer_value();
		let arguments = run
			.get_nth_param(1)
			.expect("arguments")
			.into_pointer_value();
		let frame = run.get_nth_param(2).expect("a frame").into_pointer_value();
		let mut values: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
		let receiver = function.params.first().map(|(repr, _)| *repr);
		match (function.by_pointer.first(), receiver) {
			(Some(true), _) => values.push(state.into()),
			(_, Some(repr)) => {
				values.push(builder.build_load(self.llvm_type(repr), state, "")?.into());
			}
			_ => return Err(Error::Llvm("a worker method without its receiver".into())),
		}
		for index in 0..block_type.count_fields() {
			let field_type = block_type.get_field_type_at_index(index).expect("a field");
			let at = builder.build_struct_gep(block_type, arguments, index, "")?;
			values.push(builder.build_load(field_type, at, "")?.into());
		}
		let call = builder.build_call(function.value, &values, "")?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		{
			let raw = self.to_promise_raw(
				&builder,
				call.try_as_basic_value().basic(),
				function.ret,
				function.throws,
			)?;
			let promise_type = self.promise_type(function.ret, function.throws);
			let promise = unsafe {
				builder.build_in_bounds_gep(
					self.context.i8_type(),
					frame,
					&[self.usize.const_int(TASK_AT, false)],
					"",
				)?
			};
			let result = builder.build_struct_gep(promise_type, promise, 1, "")?;
			builder.build_store(result, raw)?;
		}
		builder.build_call(drop_arguments, &[arguments.into()], "")?;
		builder.build_return(None)?;
		Ok(run)
	}

	/// `fa.worker_call.dispose(ptr frame)` of a result shape: drops the
	/// result of a call when it wasn't taken, then frees the call
	/// (`fa_call_free`). Called by the runtime (the C convention).
	fn worker_dispose(&mut self, ret: Repr, throws: bool) -> Result<FunctionValue<'ctx>, Error> {
		let key = format!(
			"dispose.{ret:?}.{throws}.{:?}",
			self.bindings.borrow().len()
		);
		if let Some(function) = self.tasks.finished.get(&key) {
			return Ok(*function);
		}
		let pointer = self.pointer_type();
		let function = self.module.add_function(
			"fa.worker_call.dispose",
			self.context.void_type().fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		self.tasks.finished.insert(key, function);
		let resume = self.builder.get_insert_block().expect("a block");
		let outer = self.frame.take();
		self.begin_function(
			function,
			Repr::Void,
			outer.as_ref().and_then(|frame| frame.owner),
		);
		let result = (|| -> Result<(), Error> {
			let block = function
				.get_nth_param(0)
				.expect("a block")
				.into_pointer_value();
			let promise_type = self.promise_type(ret, throws);
			let promise = self.promise_of(block)?;
			if self.raw_type(ret, throws).is_some() && (self.needs_drop(ret) || throws) {
				let header = self
					.builder
					.build_struct_gep(promise_type, promise, 0, "")?;
				self.drop_promise_result(header, promise, promise_type, ret, throws)?;
			}
			self.call_runtime("fa_call_free", &[block.into()])?;
			self.builder.build_return(None)?;
			Ok(())
		})();
		let ended = self.end_function();
		self.frame = outer;
		self.builder.position_at_end(resume);
		result?;
		ended?;
		Ok(function)
	}
}

// ── async function values (design/async.md §2.2) ─────────────────────────

impl<'ctx> Lowering<'_, 'ctx> {
	/// The entry of a function value: `sync` (its function, taking the
	/// environment first), or, for an `async` function of an asynchronous
	/// program, a constant table `{ sync, async }` whose address is tagged
	/// with its low bit. A value of an `async` function type stays two words
	/// (`{ entry, environment }`); a call through it reads the tag (a value
	/// of a function that isn't `async` has none: awaiting it calls it).
	/// `thunk`: `sync` is the thunk of a declaration, so the async entry is
	/// a thunk too (it skips the environment).
	pub(crate) fn async_value_entry(
		&mut self,
		function: &Function<'ctx>,
		sync: PointerValue<'ctx>,
		thunk: bool,
	) -> Result<PointerValue<'ctx>, Error> {
		if !self.tasks.asynchronous || !self.is_async_function(function) {
			return Ok(sync);
		}
		let instance = self.async_instance(function)?;
		let entry = if thunk {
			let name = format!("{}.thunk", instance.get_name().to_string_lossy());
			match self.module.get_function(&name) {
				Some(existing) => existing,
				None => {
					let mut parameters: Vec<BasicMetadataTypeEnum<'ctx>> =
						vec![self.pointer_type().into()];
					parameters.extend(instance.get_type().get_param_types());
					let thunk_type = self.pointer_type().fn_type(&parameters, false);
					let thunk =
						self.module
							.add_function(&name, thunk_type, Some(Linkage::Internal));
					thunk.set_call_conventions(FAST_CALLING_CONVENTION);
					self.runtime.add_attributes(thunk, &["nounwind"]);
					let builder = self.context.create_builder();
					builder.position_at_end(self.context.append_basic_block(thunk, "entry"));
					let arguments: Vec<BasicMetadataValueEnum<'ctx>> =
						thunk.get_param_iter().skip(1).map(Into::into).collect();
					let call = builder.build_call(instance, &arguments, "")?;
					call.set_call_convention(FAST_CALLING_CONVENTION);
					let frame = call.try_as_basic_value().basic().expect("a frame");
					builder.build_return(Some(&frame))?;
					thunk
				}
			}
		} else {
			instance
		};
		let pointer = self.pointer_type();
		let table_type = self
			.context
			.struct_type(&[pointer.into(), pointer.into()], false);
		let table = self.module.add_global(table_type, None, "fa.async_entries");
		table.set_initializer(&table_type.const_named_struct(&[
			sync.into(),
			entry.as_global_value().as_pointer_value().into(),
		]));
		table.set_constant(true);
		table.set_linkage(Linkage::Private);
		table.set_alignment(16);
		// SAFETY: one byte into a global of 16 bytes
		Ok(unsafe {
			table.as_pointer_value().const_gep(
				self.context.i8_type(),
				&[self.context.i64_type().const_int(1, false)],
			)
		})
	}

	/// A call through a value of an `async` function type in an asynchronous
	/// program: awaited in an async instance, the async entry runs and the
	/// caller waits for its frame; started (a kept task), its frame is the
	/// task. Otherwise, and for a value without async entry, the synchronous
	/// entry is called (`None`: the caller does it, its mode kept).
	pub(crate) fn async_value_call(
		&mut self,
		closure: Val<'ctx>,
		ty: TypeId,
		arguments: &[Val<'ctx>],
		node: NodeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		if !self.tasks.asynchronous {
			return Ok(None);
		}
		let TypeKind::Function(sig) = self.types().kind(ty) else {
			return Ok(None);
		};
		if !self.types().sig(sig).is_async {
			return Ok(None);
		}
		let mode = self.tasks.call_modes.remove(&node);
		let (mut params, ret, throws) = self.value_signature(ty, node)?;
		if let Some(arity) = self.declared_arity(node) {
			params.truncate(arity);
		}
		let pair = closure.value.expect("a function value").into_struct_value();
		let entry = self
			.builder
			.build_extract_value(pair, 0, "")?
			.into_pointer_value();
		let environment = self.builder.build_extract_value(pair, 1, "")?;
		let mut lowered: Vec<BasicMetadataValueEnum<'ctx>> = vec![environment.into()];
		for (repr, argument) in params.iter().zip(arguments) {
			let argument = self.convert(*argument, *repr, node.index())?;
			lowered.push(argument.value.expect("an argument").into());
		}
		let pointer = self.pointer_type();
		let address = self.builder.build_ptr_to_int(entry, self.usize, "")?;
		let tag = self
			.builder
			.build_and(address, self.usize.const_int(1, false), "")?;
		let tagged = self.builder.build_int_compare(
			IntPredicate::NE,
			tag,
			self.usize.const_zero(),
			"tagged",
		)?;
		let table = unsafe {
			self.builder.build_gep(
				self.context.i8_type(),
				entry,
				&[self.context.i64_type().const_all_ones()],
				"table",
			)?
		};
		let sync_type = self.indirect_type(&params, ret, throws);
		let mut async_params = sync_type.get_param_types();
		async_params.push(self.context.i32_type().into());
		let async_type = pointer.fn_type(&async_params, false);
		let with_async = self.append_block("async_entry");
		let without = self.append_block("sync_entry");
		let merged = self.append_block("entry_called");
		let coroutine = self.in_coroutine();
		match mode {
			Some(CallMode::Await) if coroutine => {
				let raw_type = self.raw_type(ret, throws);
				let slot = match raw_type {
					Some(raw_type) => Some(self.alloca_type(raw_type, "result")?),
					None => None,
				};
				self.builder
					.build_conditional_branch(tagged, with_async, without)?;
				self.builder.position_at_end(with_async);
				let function = self.load_entry(table, 1)?;
				let mut arguments = lowered.clone();
				arguments.push(self.context.i32_type().const_zero().into());
				let call = self
					.builder
					.build_indirect_call(async_type, function, &arguments, "task")?;
				call.set_call_convention(FAST_CALLING_CONVENTION);
				let frame = call
					.try_as_basic_value()
					.basic()
					.expect("a frame")
					.into_pointer_value();
				self.await_frame(frame, Some(node))?;
				let raw = self.take_result(frame, ret, throws)?;
				if let (Some(slot), Some(raw)) = (slot, raw) {
					self.builder.build_store(slot, raw)?;
				}
				self.builder.build_unconditional_branch(merged)?;
				self.builder.position_at_end(without);
				let call = self
					.builder
					.build_indirect_call(sync_type, entry, &lowered, "")?;
				call.set_call_convention(FAST_CALLING_CONVENTION);
				if let (Some(slot), Some(raw)) = (slot, call.try_as_basic_value().basic()) {
					self.builder.build_store(slot, raw)?;
				}
				self.builder.build_unconditional_branch(merged)?;
				self.builder.position_at_end(merged);
				let raw = match (slot, raw_type) {
					(Some(slot), Some(raw_type)) => {
						Some(self.builder.build_load(raw_type, slot, "")?)
					}
					_ => None,
				};
				Ok(Some(self.call_result(raw, ret, throws)?))
			}
			Some(CallMode::Start {
				detached: false,
				task,
				immediate,
			}) if self.start_kind() == Some(true) => {
				let slot = self.alloca_type(pointer.into(), "task")?;
				self.builder
					.build_conditional_branch(tagged, with_async, without)?;
				self.builder.position_at_end(with_async);
				let function = self.load_entry(table, 1)?;
				let mut arguments = lowered.clone();
				let lazy = if immediate { 0 } else { MODE_LAZY };
				arguments.push(self.context.i32_type().const_int(lazy, false).into());
				let call = self
					.builder
					.build_indirect_call(async_type, function, &arguments, "task")?;
				call.set_call_convention(FAST_CALLING_CONVENTION);
				let frame = call
					.try_as_basic_value()
					.basic()
					.expect("a frame")
					.into_pointer_value();
				if !immediate {
					let kept = self.context.i32_type().const_zero();
					self.call_runtime("fa_task_spawn", &[frame.into(), kept.into()])?;
				}
				self.builder.build_store(slot, frame)?;
				self.builder.build_unconditional_branch(merged)?;
				self.builder.position_at_end(without);
				let call = self
					.builder
					.build_indirect_call(sync_type, entry, &lowered, "")?;
				call.set_call_convention(FAST_CALLING_CONVENTION);
				let block = self.finished_block(call.try_as_basic_value().basic(), ret, throws)?;
				self.builder.build_store(slot, block)?;
				self.builder.build_unconditional_branch(merged)?;
				self.builder.position_at_end(merged);
				let frame = self.builder.build_load(pointer, slot, "")?;
				Ok(Some(self.fresh(Val::new(Repr::Task(task), frame))))
			}
			_ => {
				// a blocking call: the synchronous entry (start runs it at once)
				if let Some(mode) = mode {
					self.tasks.call_modes.insert(node, mode);
				}
				let slot = self.alloca_type(pointer.into(), "entry")?;
				self.builder.build_store(slot, entry)?;
				self.builder
					.build_conditional_branch(tagged, with_async, without)?;
				self.builder.position_at_end(with_async);
				let function = self.load_entry(table, 0)?;
				self.builder.build_store(slot, function)?;
				self.builder.build_unconditional_branch(merged)?;
				self.builder.position_at_end(without);
				self.builder.build_unconditional_branch(merged)?;
				self.builder.position_at_end(merged);
				let function = self
					.builder
					.build_load(pointer, slot, "")?
					.into_pointer_value();
				let call = self
					.builder
					.build_indirect_call(sync_type, function, &lowered, "")?;
				call.set_call_convention(FAST_CALLING_CONVENTION);
				Ok(Some(self.call_result(
					call.try_as_basic_value().basic(),
					ret,
					throws,
				)?))
			}
		}
	}

	/// An entry of the table of an async function value.
	fn load_entry(
		&self,
		table: PointerValue<'ctx>,
		index: u64,
	) -> Result<PointerValue<'ctx>, Error> {
		let pointer = self.pointer_type();
		let at = unsafe {
			self.builder.build_in_bounds_gep(
				pointer,
				table,
				&[self.usize.const_int(index, false)],
				"",
			)?
		};
		Ok(self
			.builder
			.build_load(pointer, at, "")?
			.into_pointer_value())
	}
}

// ── worker pools (`WorkerPool(T)`, `design/decisions.md`) ──────────────────

/// Where a pool's copy of the state is in its block (`POOL_STATE`).
const POOL_STATE: u64 = 384;

impl<'ctx> Lowering<'_, 'ctx> {
	/// `WorkerPool(T) { count, state, timeout }`: a pool block holding a copy
	/// of the state (given, or `T {}` recorded on the literal's type), then
	/// its workers, each made from a copy of it.
	pub(crate) fn worker_pool_literal(
		&mut self,
		node: NodeId,
		info: &fa_analyzer::analysis::ObjectInfo,
	) -> Result<Val<'ctx>, Error> {
		let TypeKind::Sync(_, state_type) = self.types().kind(info.ty) else {
			return Err(self.unsupported(node.index(), "this pool"));
		};
		let mut given: FxHashMap<String, NodeId> = FxHashMap::default();
		for &(name, value) in &info.fields {
			if let fa_analyzer::analysis::FieldValue::Given(value) = value {
				given.insert(self.types().syms.resolve(name).to_string(), value);
			}
		}
		let i64 = self.context.i64_type();
		let integer = |this: &mut Self, name: &str| -> Result<Option<IntValue<'ctx>>, Error> {
			match given.get(name) {
				Some(&value) => {
					let lowered = this.expr(value)?;
					if lowered.repr == Repr::Never {
						return Ok(None);
					}
					Ok(Some(
						this.convert(lowered, Repr::Int(IntRepr::I64), value.index())?
							.int(),
					))
				}
				None => Ok(Some(i64.const_zero())),
			}
		};
		let Some(count) = integer(self, "count")? else {
			return Ok(Val::never());
		};
		let Some(timeout) = integer(self, "timeout")? else {
			return Ok(Val::never());
		};
		let Some(max_queue) = integer(self, "maxQueueSize")? else {
			return Ok(Val::never());
		};
		let state = match given.get("state") {
			Some(&value) => self.expr(value)?,
			None => {
				let View::TypedObject { target, .. } = self.view(node) else {
					return Err(self.unsupported(node.index(), "this pool"));
				};
				self.object_literal(target)?
			}
		};
		if state.repr == Repr::Never {
			return Ok(state);
		}
		let repr = self.repr_of(state_type, node.index())?;
		let state = self.owned_as(state, repr, Some(node))?;
		let size = self.size_of_repr(repr)?;
		let drop = self.drop_at_function(repr, POOL_STATE)?;
		let spawn = self.pool_spawn_function(repr)?;
		let pool = self
			.runtime_value(
				"fa_workers_new",
				&[
					count.into(),
					timeout.into(),
					max_queue.into(),
					size.into(),
					drop.into(),
					spawn.as_global_value().as_pointer_value().into(),
				],
			)?
			.into_pointer_value();
		let at = self.byte_offset(pool, POOL_STATE)?;
		if let Some(raw) = state.value {
			self.builder.build_store(at, raw)?;
		}
		self.call_runtime("fa_workers_fill", &[pool.into()])?;
		Ok(self.fresh(Val::new(Repr::Sync(info.ty), pool)))
	}

	/// `fa.pool.spawn(ptr pool) -> ptr`: a new worker whose state is a copy
	/// of the pool's, its thread started.
	fn pool_spawn_function(&self, repr: Repr) -> Result<FunctionValue<'ctx>, Error> {
		let name = format!("fa.pool.spawn.{repr:?}");
		if let Some(existing) = self.module.get_function(&name) {
			return Ok(existing);
		}
		let pointer = self.pointer_type();
		let spawn = self.module.add_function(
			&name,
			pointer.fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(spawn, "entry"));
		let pool = spawn.get_nth_param(0).expect("a pool").into_pointer_value();
		let i8 = self.context.i8_type();
		let i64 = self.context.i64_type();
		let from = unsafe {
			builder.build_in_bounds_gep(i8, pool, &[i64.const_int(POOL_STATE, false)], "")?
		};
		let state = builder.build_load(self.llvm_type(repr), from, "")?;
		let copy = self.call_copy(&builder, repr, state)?;
		let size =
			inkwell::types::BasicType::size_of(&self.llvm_type(repr)).expect("a sized state");
		let size = builder.build_int_cast(size, i64, "")?;
		let drop = self.drop_at_function(repr, WORKER_STATE)?;
		let worker = builder
			.build_call(
				self.runtime.function("fa_worker_new")?,
				&[size.into(), drop.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a worker")
			.into_pointer_value();
		let to = unsafe {
			builder.build_in_bounds_gep(i8, worker, &[i64.const_int(WORKER_STATE, false)], "")?
		};
		builder.build_store(to, copy)?;
		builder.build_call(
			self.runtime.function("fa_worker_start_in_pool")?,
			&[worker.into(), pool.into()],
			"",
		)?;
		builder.build_return(Some(&worker))?;
		Ok(spawn)
	}

	/// `pool.submit(work)`: a job queued in the pool, which a worker's thread
	/// takes and runs (the work called with the worker); its frame is the
	/// task. A place in the queue first (`maxQueueSize`): awaited (in an
	/// async instance), the task waits for it; else the thread does.
	pub(crate) fn pool_submit(
		&mut self,
		node: NodeId,
		info: &CallInfo,
		awaited: bool,
	) -> Result<Val<'ctx>, Error> {
		let Some(work) = info.arguments.iter().find_map(|argument| match argument {
			Argument::Given(value) => Some(*value),
			_ => None,
		}) else {
			return Err(self.unsupported(node.index(), "this `submit`"));
		};
		let pool = self.receiver(node)?;
		if pool.repr == Repr::Never {
			return Ok(pool);
		}
		let pool = pool.value.expect("a pool").into_pointer_value();
		// a place in the queue, given back when the call takes a worker
		if awaited && self.in_coroutine() {
			let handle = self.coroutine().expect("a coroutine").handle;
			let wait = self.words(WAIT_WORDS, "room")?;
			let status = self
				.runtime_value(
					"fa_workers_reserve",
					&[pool.into(), wait.into(), handle.into()],
				)?
				.into_int_value();
			let ready = self.builder.build_int_compare(
				IntPredicate::EQ,
				status,
				self.context.i32_type().const_int(AWAIT_READY, false),
				"",
			)?;
			let full = self.append_block("queue_full");
			let room = self.append_block("queue_room");
			self.builder.build_conditional_branch(ready, room, full)?;
			self.builder.position_at_end(full);
			self.suspend(Some(node), |this| {
				this.call_runtime("fa_workers_unreserve", &[pool.into(), wait.into()])
			})?;
			self.builder.build_unconditional_branch(room)?;
			self.builder.position_at_end(room);
		} else {
			self.call_runtime("fa_workers_reserve_now", &[pool.into()])?;
		}
		let closure = self.expr(work)?;
		if closure.repr == Repr::Never {
			return Ok(closure);
		}
		let closure = self.owned_as(closure, closure.repr, Some(work))?;
		let task = self.type_of(node);
		let (ret, throws) = self.task_layout(task, node)?;
		let work_type = self.type_of(work);
		// the job: a frame laid out like a task's, and room for its result
		let promise_type = self.promise_type(ret, throws);
		let frame_size = self.finished_size(promise_type);
		let i64 = self.context.i64_type();
		let frame_size = self.builder.build_int_cast(frame_size, i64, "")?;
		let raw_size = {
			let raw_type = self.promise_raw_type(ret);
			let size = inkwell::types::BasicType::size_of(&raw_type).expect("a sized result");
			self.builder.build_int_cast(size, i64, "")?
		};
		let frame = self
			.runtime_value("fa_job_new", &[frame_size.into(), raw_size.into()])?
			.into_pointer_value();
		let pointer = self.pointer_type();
		let (resume, destroy) = self.job_frame_functions()?;
		self.builder
			.build_store(frame, resume.as_global_value().as_pointer_value())?;
		let destroy_slot = unsafe {
			self.builder.build_in_bounds_gep(
				pointer,
				frame,
				&[self.usize.const_int(1, false)],
				"",
			)?
		};
		self.builder
			.build_store(destroy_slot, destroy.as_global_value().as_pointer_value())?;
		// the closure, owned by the job
		let block = self.allocate(self.usize.const_int(16, false))?;
		self.builder
			.build_store(block, closure.value.expect("a function"))?;
		let (run, drop_closure) = self.job_run_functions(node, work_type, ret, throws)?;
		let dispose = self.worker_dispose(ret, throws)?;
		let (timeout_error, drop_result) = self.job_result_functions(ret, throws)?;
		let global = |function: FunctionValue<'ctx>| -> BasicMetadataValueEnum<'ctx> {
			function.as_global_value().as_pointer_value().into()
		};
		self.call_runtime(
			"fa_workers_enqueue",
			&[
				pool.into(),
				frame.into(),
				global(run),
				block.into(),
				global(drop_closure),
				global(dispose),
				global(timeout_error),
				global(drop_result),
			],
		)?;
		Ok(self.fresh(Val::new(Repr::Task(task), frame)))
	}

	/// The resume and destroy functions of the frames of jobs (`fastcc`):
	/// they call `fa_job_resume` and `fa_job_destroy`.
	fn job_frame_functions(&self) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		let make = |name: &str, target: &str| -> Result<FunctionValue<'ctx>, Error> {
			if let Some(function) = self.module.get_function(name) {
				return Ok(function);
			}
			let pointer = self.pointer_type();
			let function = self.module.add_function(
				name,
				self.context.void_type().fn_type(&[pointer.into()], false),
				Some(Linkage::Internal),
			);
			function.set_call_conventions(FAST_CALLING_CONVENTION);
			self.runtime.add_attributes(function, &["nounwind"]);
			let builder = self.context.create_builder();
			builder.position_at_end(self.context.append_basic_block(function, "entry"));
			let frame = function.get_nth_param(0).expect("a frame");
			builder.build_call(self.runtime.function(target)?, &[frame.into()], "")?;
			builder.build_return(None)?;
			Ok(function)
		};
		Ok((
			make("fa.job.resume", "fa_job_resume")?,
			make("fa.job.destroy", "fa_job_destroy")?,
		))
	}

	/// `fa.job.run(ptr worker, ptr closure, ptr result)`: on the worker's
	/// thread, calls the submitted function (its synchronous entry) with the
	/// worker, writes the task's raw result, then drops the closure; and
	/// `fa.job.drop_closure(ptr closure)` (a job cancelled while queued).
	fn job_run_functions(
		&mut self,
		node: NodeId,
		work_type: TypeId,
		ret: Repr,
		throws: bool,
	) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		let pointer = self.pointer_type();
		let (params, work_ret, work_throws) = self.value_signature(work_type, node)?;
		let sync_type = self.indirect_type(&params, work_ret, work_throws);
		let closure_repr = Repr::Function(work_type);
		let closure_type = self.closure_type();
		// drops the closure and frees its block
		let drop_closure = self.module.add_function(
			"fa.job.drop_closure",
			self.context.void_type().fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		{
			let builder = self.context.create_builder();
			builder.position_at_end(self.context.append_basic_block(drop_closure, "entry"));
			let block = drop_closure
				.get_nth_param(0)
				.expect("a closure")
				.into_pointer_value();
			let closure = builder.build_load(closure_type, block, "")?;
			self.call_drop(&builder, closure_repr, closure)?;
			builder.build_call(
				self.runtime.free()?,
				&[
					block.into(),
					self.usize.const_int(16, false).into(),
					self.usize.const_int(16, false).into(),
				],
				"",
			)?;
			builder.build_return(None)?;
		}
		let run = self.module.add_function(
			"fa.job.run",
			self.context
				.void_type()
				.fn_type(&[pointer.into(), pointer.into(), pointer.into()], false),
			Some(Linkage::Internal),
		);
		let builder = self.context.create_builder();
		let entry_block = self.context.append_basic_block(run, "entry");
		let tagged_block = self.context.append_basic_block(run, "tagged");
		let call_block = self.context.append_basic_block(run, "call");
		builder.position_at_end(entry_block);
		let worker = run.get_nth_param(0).expect("a worker");
		let block = run
			.get_nth_param(1)
			.expect("a closure")
			.into_pointer_value();
		let result = run.get_nth_param(2).expect("a result").into_pointer_value();
		let closure = builder
			.build_load(closure_type, block, "")?
			.into_struct_value();
		let entry = builder
			.build_extract_value(closure, 0, "")?
			.into_pointer_value();
		let environment = builder.build_extract_value(closure, 1, "")?;
		// an async function's value: its synchronous entry is in its table
		let address = builder.build_ptr_to_int(entry, self.usize, "")?;
		let tag = builder.build_and(address, self.usize.const_int(1, false), "")?;
		let tagged =
			builder.build_int_compare(IntPredicate::NE, tag, self.usize.const_zero(), "")?;
		builder.build_conditional_branch(tagged, tagged_block, call_block)?;
		builder.position_at_end(tagged_block);
		let table = unsafe {
			builder.build_gep(
				self.context.i8_type(),
				entry,
				&[self.context.i64_type().const_all_ones()],
				"",
			)?
		};
		let sync_entry = builder.build_load(pointer, table, "")?.into_pointer_value();
		builder.build_unconditional_branch(call_block)?;
		builder.position_at_end(call_block);
		let function = builder.build_phi(pointer, "entry")?;
		function.add_incoming(&[(&entry, entry_block), (&sync_entry, tagged_block)]);
		let call = builder.build_indirect_call(
			sync_type,
			function.as_basic_value().into_pointer_value(),
			&[environment.into(), worker.into()],
			"",
		)?;
		call.set_call_convention(FAST_CALLING_CONVENTION);
		// the task's raw result, as a promise holds it: the function's, with
		// no error when it can't throw
		let _ = throws;
		let raw = self.to_promise_raw(
			&builder,
			call.try_as_basic_value().basic(),
			ret,
			work_throws,
		)?;
		builder.build_store(result, raw)?;
		builder.build_call(drop_closure, &[block.into()], "")?;
		builder.build_return(None)?;
		Ok((run, drop_closure))
	}

	/// `fa.job.timeout_error(ptr frame)`: writes `WorkerTimeout` as the raw
	/// result of a job's promise; `fa.job.drop_result(ptr result)`: drops a
	/// raw result (a job abandoned). Called by the runtime.
	fn job_result_functions(
		&mut self,
		ret: Repr,
		throws: bool,
	) -> Result<(FunctionValue<'ctx>, FunctionValue<'ctx>), Error> {
		let pointer = self.pointer_type();
		let make = |this: &mut Self,
		            name: &str,
		            body: &dyn Fn(&mut Self, PointerValue<'ctx>) -> Result<(), Error>|
		 -> Result<FunctionValue<'ctx>, Error> {
			let function = this.module.add_function(
				name,
				this.context.void_type().fn_type(&[pointer.into()], false),
				Some(Linkage::Internal),
			);
			let resume = this.builder.get_insert_block().expect("a block");
			let outer = this.frame.take();
			this.begin_function(
				function,
				Repr::Void,
				outer.as_ref().and_then(|frame| frame.owner),
			);
			let parameter = function
				.get_nth_param(0)
				.expect("a pointer")
				.into_pointer_value();
			let result = body(this, parameter).and_then(|()| {
				this.builder.build_return(None)?;
				Ok(())
			});
			let ended = this.end_function();
			this.frame = outer;
			this.builder.position_at_end(resume);
			result?;
			ended?;
			Ok(function)
		};
		let timeout_error = make(self, "fa.job.timeout_error", &|this, frame| {
			// a pool without `timeout` never times out: its tasks can't fail
			if !throws || this.raw_type(ret, throws).is_none() {
				return Ok(());
			}
			let error = this
				.runtime_value("fa_error_worker_timeout", &[])?
				.into_pointer_value();
			let raw = this.raw_of_error(error, ret)?;
			let promise_type = this.promise_type(ret, throws);
			let promise = this.promise_of(frame)?;
			let at = this
				.builder
				.build_struct_gep(promise_type, promise, 1, "")?;
			this.builder.build_store(at, raw)?;
			Ok(())
		})?;
		let drop_result = make(self, "fa.job.drop_result", &|this, result| {
			let raw = this
				.builder
				.build_load(this.promise_raw_type(ret), result, "")?;
			match this.raw_of_promise(raw, ret, throws)? {
				Some(raw) => this.drop_raw(raw, ret, throws),
				None => Ok(()),
			}
		})?;
		Ok((timeout_error, drop_result))
	}

	/// `pool.shutdown()`.
	pub(crate) fn pool_shutdown(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let pool = self.receiver(node)?;
		if pool.repr == Repr::Never {
			return Ok(pool);
		}
		self.call_runtime("fa_workers_shutdown", &[pool.value.expect("a pool").into()])?;
		Ok(Val::void())
	}
}
