//! Concurrency natively (advanced brief §14, the implementation notes of
//! `documentation/advanced/concurrency.md`): the thread-safe handles of
//! `std.concurrency` and `for parallel`. The runtime part is the
//! concurrency section of `runtime/fa_runtime.c`.
//!
//! - **Handles** (`AtomicInteger`, `AtomicBoolean`, `Mutex(T)`,
//!   `Channel(T)`): `Repr::Sync`, a pointer to a reference-counted block
//!   (`fa_shared_retain` copies, `fa_shared_release` drops). An atomic is
//!   an `i64` at [`ATOMIC_VALUE`], read and written with atomic
//!   instructions (sequentially consistent: the requested ordering is
//!   never weaker). A mutex holds its value at [`MUTEX_VALUE`]; a channel
//!   holds a ring of values, moved in and out by bytes, dropped with the
//!   channel by a generated function.
//! - **A guard** is the pointer to its mutex's block (it holds a handle):
//!   `guard.value` is the place at [`MUTEX_VALUE`], its drop unlocks.
//! - **A worker** (`*.worker.fa`) is its state: its calls are blocking
//!   (D31), so running them on the caller's thread is the same as waiting
//!   for the worker's thread; two threads can't call a method changing its
//!   state at once (`CONC002`). `stop()` does nothing more than the end of
//!   the handle's scope. Its own thread comes with `start` and `await`.
//! - **`for parallel`** over a range literal or an array: the body is
//!   outlined into `ptr fa.parallel(ptr environment, i64 start, i64 end,
//!   ptr failed)`, which runs the iterations `[start, end)` and returns the
//!   error of the first one that throws. The environment holds pointers to
//!   the locals the body uses (the writes are disjoint: the analyzer proved
//!   it), the array or the start of the range. `fa_parallel_for` runs it
//!   on the runtime's pool; on the freestanding target, which has no
//!   threads, the loop calls it once, in order.

use fa_analyzer::analysis::{Argument, CallInfo, FieldValue, ObjectInfo};
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{NodeList, View};
use fa_analyzer::types::{SyncKind, TypeKind};
use inkwell::module::Linkage;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::{AtomicOrdering, AtomicRMWBinOp, IntPredicate};

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};
use crate::lower::{LoopFrame, Lowering, Slot};
use crate::target::Target;

/// The offset of the value of an atomic in its block (`FaAtomicBlock`).
const ATOMIC_VALUE: u64 = 32;
/// The offset of the value of a mutex in its block (`FaMutex`).
const MUTEX_VALUE: u64 = 176;

const SEQUENTIAL: AtomicOrdering = AtomicOrdering::SequentiallyConsistent;

/// What a parallel loop iterates over.
enum Iterated<'ctx> {
	/// `a..b`, `a..=b`: the first value.
	Range(IntValue<'ctx>),
	/// An array: its pointer.
	Array(PointerValue<'ctx>, TypeId),
	/// A `GrowingArray`: its segments (`fa_segmented_at` finds each item).
	Growing(PointerValue<'ctx>, TypeId),
	/// A pool or a columnar: an array of the addresses of its elements, in
	/// slot order (taken before the loop), and how an element is held.
	Elements(PointerValue<'ctx>, Repr),
}

/// What the body of a parallel loop reads its items from (`Iterated`
/// without the values of the function around it).
#[derive(Copy, Clone)]
enum Items {
	Range,
	Array(TypeId),
	Growing(TypeId),
	Elements(Repr),
}

/// The places of the function around a parallel body that it uses: its
/// locals, the values given with `provide`, the requirements it was given.
struct Outer<'ctx> {
	locals: Vec<(SymbolId, Slot<'ctx>)>,
	provided: Vec<(NodeId, Slot<'ctx>)>,
	requirements: Vec<(PointerValue<'ctx>, TypeId)>,
}

impl Outer<'_> {
	/// The number of places, the first fields of the environment.
	fn len(&self) -> usize {
		self.locals.len() + self.provided.len() + self.requirements.len()
	}
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The kind and item of a type of `std.concurrency`.
	pub(crate) fn sync_kind(&self, ty: TypeId) -> Option<(SyncKind, TypeId)> {
		match self.types().kind(ty) {
			TypeKind::Sync(kind, item) => Some((kind, item)),
			_ => None,
		}
	}

	/// The drop of a handle: its release, or the unlock of a guard.
	pub(crate) fn build_sync_drop(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		ty: TypeId,
		value: BasicValueEnum<'ctx>,
	) -> Result<(), Error> {
		let name = match self.sync_kind(ty) {
			Some((SyncKind::Guard, _)) => "fa_mutex_unlock",
			_ => "fa_shared_release",
		};
		builder.build_call(self.runtime.function(name)?, &[value.into()], "")?;
		Ok(())
	}

	/// The copy of a handle: the same block, one more handle.
	pub(crate) fn build_sync_copy(
		&self,
		builder: &inkwell::builder::Builder<'ctx>,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		Ok(builder
			.build_call(
				self.runtime.function("fa_shared_retain")?,
				&[value.into()],
				"",
			)?
			.try_as_basic_value()
			.basic()
			.expect("a handle"))
	}

	/// `fa.drop_at.<shape>(ptr)`: drops the value of this representation
	/// stored `offset` bytes after the pointer; null when nothing to drop.
	pub(crate) fn drop_at_function(
		&self,
		repr: Repr,
		offset: u64,
	) -> Result<PointerValue<'ctx>, Error> {
		if !self.needs_drop(repr) {
			return Ok(self.pointer_type().const_null());
		}
		let name = format!("fa.drop_at.{offset}.{repr:?}");
		if let Some(function) = self.module.get_function(&name) {
			return Ok(function.as_global_value().as_pointer_value());
		}
		let pointer = self.pointer_type();
		let function = self.module.add_function(
			&name,
			self.context.void_type().fn_type(&[pointer.into()], false),
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		let builder = self.context.create_builder();
		builder.position_at_end(self.context.append_basic_block(function, "entry"));
		let base = function
			.get_nth_param(0)
			.expect("a pointer")
			.into_pointer_value();
		let at = unsafe {
			builder.build_in_bounds_gep(
				self.context.i8_type(),
				base,
				&[self.context.i64_type().const_int(offset, false)],
				"",
			)?
		};
		let value = builder.build_load(self.llvm_type(repr), at, "")?;
		self.call_drop(&builder, repr, value)?;
		builder.build_return(None)?;
		Ok(function.as_global_value().as_pointer_value())
	}

	/// A pointer `offset` bytes into a block.
	pub(crate) fn byte_offset(
		&self,
		block: PointerValue<'ctx>,
		offset: u64,
	) -> Result<PointerValue<'ctx>, Error> {
		Ok(unsafe {
			self.builder.build_in_bounds_gep(
				self.context.i8_type(),
				block,
				&[self.context.i64_type().const_int(offset, false)],
				"",
			)?
		})
	}

	/// The size of the values of a representation.
	pub(crate) fn size_of_repr(&self, repr: Repr) -> Result<IntValue<'ctx>, Error> {
		let size = self.llvm_type(repr).size_of().expect("a sized value");
		Ok(self
			.builder
			.build_int_cast(size, self.context.i64_type(), "")?)
	}

	/// `AtomicInteger { value = 0 }`, `Mutex(T) { value = … }`,
	/// `Channel(T) { capacity = 16 }`: a new block, one handle.
	pub(crate) fn sync_literal(
		&mut self,
		node: NodeId,
		info: &ObjectInfo,
	) -> Result<Val<'ctx>, Error> {
		let Some((kind, item)) = self.sync_kind(info.ty) else {
			return Err(self.unsupported(node.index(), "this value"));
		};
		let given = info.fields.iter().find_map(|(_, value)| match value {
			FieldValue::Given(value) => Some(*value),
			_ => None,
		});
		let i64 = self.context.i64_type();
		let repr = Repr::Sync(info.ty);
		let block = match kind {
			SyncKind::AtomicInteger | SyncKind::AtomicBoolean => {
				let Some(value) = given else {
					return Err(self.unsupported(node.index(), "an atomic without its value"));
				};
				let lowered = self.expr(value)?;
				if lowered.repr == Repr::Never {
					return Ok(lowered);
				}
				let raw = if kind == SyncKind::AtomicBoolean {
					let flag = self.convert(lowered, Repr::Bool, value.index())?.int();
					self.builder.build_int_z_extend(flag, i64, "")?
				} else {
					self.convert(lowered, Repr::Int(IntRepr::I64), value.index())?
						.int()
				};
				self.call_value(self.runtime.function("fa_atomic_new")?, &[raw.into()])?
					.into_pointer_value()
			}
			SyncKind::Mutex => {
				let Some(value) = given else {
					return Err(self.unsupported(node.index(), "a mutex without its value"));
				};
				let item_repr = self.item_repr(item, node)?;
				let lowered = self.expr(value)?;
				if lowered.repr == Repr::Never {
					return Ok(lowered);
				}
				let lowered = self.owned_as(lowered, item_repr, Some(value))?;
				let size = self.size_of_repr(item_repr)?;
				// the block drops its value with the last handle
				let drop = self.drop_at_function(item_repr, MUTEX_VALUE)?;
				let block = self
					.call_value(
						self.runtime.function("fa_mutex_new")?,
						&[size.into(), drop.into()],
					)?
					.into_pointer_value();
				let at = self.byte_offset(block, MUTEX_VALUE)?;
				if let Some(raw) = lowered.value {
					self.builder.build_store(at, raw)?;
				}
				block
			}
			SyncKind::Channel => {
				let capacity = match given {
					Some(value) => {
						let lowered = self.expr(value)?;
						if lowered.repr == Repr::Never {
							return Ok(lowered);
						}
						self.convert(lowered, Repr::Int(IntRepr::I64), value.index())?
							.int()
					}
					None => i64.const_all_ones(),
				};
				let item_repr = self.item_repr(item, node)?;
				let size = self.size_of_repr(item_repr)?;
				let drop = self.drop_at_function(item_repr, 0)?;
				self.call_value(
					self.runtime.function("fa_channel_new")?,
					&[capacity.into(), size.into(), drop.into()],
				)?
				.into_pointer_value()
			}
			// a socket (io.rs)
			SyncKind::TcpListener => return self.listener_literal(node, info),
			// a pool of workers (tasks.rs)
			SyncKind::WorkerPool => return self.worker_pool_literal(node, info),
			SyncKind::Guard | SyncKind::Ordering | SyncKind::Worker | SyncKind::TcpStream => {
				return Err(self.unsupported(node.index(), "this value"));
			}
		};
		Ok(self.fresh(Val::new(repr, block)))
	}

	/// The place of `guard.value`: the value in the mutex's block.
	pub(crate) fn guard_value_slot(
		&self,
		guard: PointerValue<'ctx>,
		guard_type: TypeId,
		node: NodeId,
	) -> Result<Slot<'ctx>, Error> {
		let Some((SyncKind::Guard, item)) = self.sync_kind(guard_type) else {
			return Err(self.unsupported(node.index(), "this place"));
		};
		let repr = self.item_repr(item, node)?;
		let pointer = self.byte_offset(guard, MUTEX_VALUE)?;
		Ok(Slot { pointer, repr })
	}

	/// `guard.value` read: the value in the mutex's block (a view of it).
	pub(crate) fn guard_value(&self, guard: Val<'ctx>, node: NodeId) -> Result<Val<'ctx>, Error> {
		let Repr::Sync(ty) = guard.repr else {
			return Err(self.unsupported(node.index(), "`value`"));
		};
		let pointer = guard.value.expect("a guard").into_pointer_value();
		let slot = self.guard_value_slot(pointer, ty, node)?;
		self.load(slot)
	}

	/// The given arguments of a built-in call, the ordering left out (the
	/// atomics are sequentially consistent).
	fn sync_arguments(&self, info: &CallInfo) -> Vec<NodeId> {
		info.arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.filter(|value| {
				!matches!(
					self.types().kind(self.type_of(*value)),
					TypeKind::Sync(SyncKind::Ordering, _)
				)
			})
			.collect()
	}

	/// A method of a thread-safe value.
	pub(crate) fn sync_method(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let receiver = self.receiver(node)?;
		if receiver.repr == Repr::Never {
			return Ok(receiver);
		}
		let Repr::Sync(receiver_type) = receiver.repr else {
			return Err(self.unsupported(node.index(), path));
		};
		let block = receiver.value.expect("a handle").into_pointer_value();
		let arguments = self.sync_arguments(info);
		let mut values = Vec::new();
		let (owner, method) = path.split_once('#').unwrap_or((path, ""));
		// the value sent is kept as is (moved into the channel)
		if owner != "Channel" {
			for &argument in &arguments {
				let value = self.expr(argument)?;
				if value.repr == Repr::Never {
					return Ok(value);
				}
				values.push((argument, value));
			}
		}
		let i64 = self.context.i64_type();
		let integer = Repr::Int(IntRepr::I64);
		match owner {
			"AtomicInteger" | "AtomicBoolean" => {
				let boolean = owner == "AtomicBoolean";
				let at = self.byte_offset(block, ATOMIC_VALUE)?;
				let raw = |this: &Self, index: usize| -> Result<IntValue<'ctx>, Error> {
					let (node, value) = values[index];
					if boolean {
						let flag = this.convert(value, Repr::Bool, node.index())?.int();
						Ok(this.builder.build_int_z_extend(flag, i64, "")?)
					} else {
						Ok(this.convert(value, integer, node.index())?.int())
					}
				};
				let result = |this: &Self, value: IntValue<'ctx>| -> Result<Val<'ctx>, Error> {
					if boolean {
						let flag = this.builder.build_int_compare(
							IntPredicate::NE,
							value,
							i64.const_zero(),
							"",
						)?;
						Ok(Val::new(Repr::Bool, flag))
					} else {
						Ok(Val::new(integer, value))
					}
				};
				match method {
					"load" => {
						let loaded = self.builder.build_load(i64, at, "")?;
						loaded
							.as_instruction_value()
							.expect("a load")
							.set_atomic_ordering(SEQUENTIAL)
							.map_err(|error| Error::Llvm(error.to_string()))?;
						loaded
							.as_instruction_value()
							.expect("a load")
							.set_alignment(8)
							.map_err(|error| Error::Llvm(error.to_string()))?;
						result(self, loaded.into_int_value())
					}
					"store" => {
						let value = raw(self, 0)?;
						let store = self.builder.build_store(at, value)?;
						store
							.set_atomic_ordering(SEQUENTIAL)
							.map_err(|error| Error::Llvm(error.to_string()))?;
						store
							.set_alignment(8)
							.map_err(|error| Error::Llvm(error.to_string()))?;
						Ok(Val::void())
					}
					"increment" | "decrement" | "add" => {
						let amount = match method {
							"increment" => i64.const_int(1, false),
							"decrement" => i64.const_all_ones(),
							_ => raw(self, 0)?,
						};
						let previous = self.builder.build_atomicrmw(
							AtomicRMWBinOp::Add,
							at,
							amount,
							SEQUENTIAL,
						)?;
						let new = self.builder.build_int_add(previous, amount, "")?;
						result(self, new)
					}
					"swap" => {
						let value = raw(self, 0)?;
						let previous = self.builder.build_atomicrmw(
							AtomicRMWBinOp::Xchg,
							at,
							value,
							SEQUENTIAL,
						)?;
						result(self, previous)
					}
					"compareAndSwap" => {
						let expected = raw(self, 0)?;
						let value = raw(self, 1)?;
						let pair = self
							.builder
							.build_cmpxchg(at, expected, value, SEQUENTIAL, SEQUENTIAL)?;
						let done = self.builder.build_extract_value(pair, 1, "")?;
						Ok(Val::new(Repr::Bool, done))
					}
					_ => Err(self.unsupported(node.index(), path)),
				}
			}
			"Mutex" => {
				let guard_type = self.type_of(node);
				match method {
					"lock" => {
						let guard = self
							.call_value(self.runtime.function("fa_mutex_lock")?, &[block.into()])?
							.into_pointer_value();
						Ok(self.fresh(Val::new(Repr::Sync(guard_type), guard)))
					}
					"tryLock" => {
						let guard = self
							.call_value(
								self.runtime.function("fa_mutex_try_lock")?,
								&[block.into()],
							)?
							.into_pointer_value();
						let repr = self.node_repr(node)?;
						let Repr::Optional(scalar) = repr else {
							return Err(self.unsupported(node.index(), path));
						};
						let present = self.builder.build_is_not_null(guard, "")?;
						let optional = self.optional_of(scalar, present, guard.into())?;
						Ok(self.fresh(Val::new(repr, optional)))
					}
					_ => Err(self.unsupported(node.index(), path)),
				}
			}
			"Channel" => self.channel_method(node, path, method, block, receiver_type, &arguments),
			_ => Err(self.unsupported(node.index(), path)),
		}
	}

	/// `{ present, value }`.
	fn optional_of(
		&self,
		scalar: Scalar,
		present: IntValue<'ctx>,
		value: BasicValueEnum<'ctx>,
	) -> Result<BasicValueEnum<'ctx>, Error> {
		self.pack_optional(&self.builder, scalar, present, value)
	}

	fn channel_method(
		&mut self,
		node: NodeId,
		path: &str,
		method: &str,
		block: PointerValue<'ctx>,
		channel_type: TypeId,
		arguments: &[NodeId],
	) -> Result<Val<'ctx>, Error> {
		let Some((_, item)) = self.sync_kind(channel_type) else {
			return Err(self.unsupported(node.index(), path));
		};
		let item_repr = self.item_repr(item, node)?;
		let i32 = self.context.i32_type();
		match method {
			"send" | "trySend" => {
				let wait = i32.const_int(u64::from(method == "send"), false);
				let Some(&argument) = arguments.first() else {
					return Err(self.unsupported(node.index(), path));
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
				let sent = self
					.call_value(
						self.runtime.function("fa_channel_send")?,
						&[block.into(), slot.pointer.into(), wait.into()],
					)?
					.into_int_value();
				let sent =
					self.builder
						.build_int_compare(IntPredicate::NE, sent, i32.const_zero(), "")?;
				// not sent: the value is still ours
				let kept = self.builder.build_not(sent, "")?;
				let mut error = None;
				if method == "send" {
					let callee_type = match self.view(node) {
						View::Call { callee, .. } => self.type_of(callee),
						_ => TypeId::UNKNOWN,
					};
					error = self.channel_closed_error(callee_type, node)?;
				}
				let not_sent = self.append_block("not_sent");
				let after = self.append_block("sent");
				self.builder
					.build_conditional_branch(kept, not_sent, after)?;
				self.builder.position_at_end(not_sent);
				let left = self.load(slot)?;
				self.emit_drop(left)?;
				match error {
					Some(error) => self.raise(error)?,
					None => {
						self.builder.build_unconditional_branch(after)?;
					}
				}
				self.builder.position_at_end(after);
				Ok(if method == "send" {
					Val::void()
				} else {
					Val::new(Repr::Bool, sent)
				})
			}
			"receive" | "tryReceive" => {
				let wait = i32.const_int(u64::from(method == "receive"), false);
				let slot = self.alloca(item_repr, "received")?;
				let got = self
					.call_value(
						self.runtime.function("fa_channel_receive")?,
						&[block.into(), slot.pointer.into(), wait.into()],
					)?
					.into_int_value();
				let present =
					self.builder
						.build_int_compare(IntPredicate::NE, got, i32.const_zero(), "")?;
				let repr = self.node_repr(node)?;
				let Repr::Optional(scalar) = repr else {
					return Err(self.unsupported(node.index(), path));
				};
				let loaded = self.load(slot)?;
				let value = self.convert(loaded, scalar.repr(), node.index())?;
				let optional = self.optional_of(scalar, present, value.value.expect("a value"))?;
				// absent: a zeroed optional (nothing to drop)
				let none = super::zero_of(self.llvm_type(repr));
				let chosen = self.builder.build_select(present, optional, none, "")?;
				Ok(self.fresh(Val::new(repr, chosen)))
			}
			"close" => {
				self.call_runtime("fa_channel_close", &[block.into()])?;
				Ok(Val::void())
			}
			_ => Err(self.unsupported(node.index(), path)),
		}
	}

	/// The `ChannelClosed` error `send` throws (a constant box), from the
	/// type of the method.
	pub(crate) fn channel_closed_error(
		&mut self,
		method_type: TypeId,
		node: NodeId,
	) -> Result<Option<PointerValue<'ctx>>, Error> {
		let TypeKind::Function(sig) = self.types().kind(method_type) else {
			return Ok(None);
		};
		let throws = self.types().sig(sig).throws;
		let nominal = match self.types().kind(throws) {
			TypeKind::NamedUnion(nominal, _) | TypeKind::Member(nominal, _) => nominal,
			_ => return Ok(None),
		};
		let error = self.build_member(nominal, 0, None, node)?;
		Ok(error.value.map(BasicValueEnum::into_pointer_value))
	}

	// ── for parallel ─────────────────────────────────────────────────────

	/// The locals declared outside a parallel body that it uses.
	fn parallel_captures(&self, node: NodeId, body: NodeId) -> Vec<SymbolId> {
		let span = self.tree.spans[node.index()];
		let mut captures = Vec::new();
		let mut stack = vec![body.index()];
		while let Some(current) = stack.pop() {
			if let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(current)) {
				let data = self.analysis.symbols.symbol(symbol);
				let variable = self.capture_has_value(symbol);
				let declared = self.tree.spans[data.decl_node.index()];
				let outside = declared.start < span.start || declared.start >= span.end;
				if variable && outside && !captures.contains(&symbol) {
					captures.push(symbol);
				}
			}
			self.tree.nodes[current].for_each_child(|child| stack.push(child));
		}
		captures.sort();
		captures
	}

	/// `for parallel item, index in iterable { … }` (`parallel(threads = n)`).
	pub(crate) fn parallel_for(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		body: NodeId,
		options: NodeList<'_>,
	) -> Result<Val<'ctx>, Error> {
		let i64 = self.context.i64_type();
		// the number of threads
		let mut threads = i64.const_zero();
		for option in options.iter() {
			if let View::Assignment {
				value: Some(value), ..
			} = self.view(option)
			{
				let lowered = self.expr(value)?;
				if lowered.repr == Repr::Never {
					return Ok(lowered);
				}
				threads = self
					.convert(lowered, Repr::Int(IntRepr::I64), value.index())?
					.int();
			}
		}
		// what is iterated, and how many times
		let (iterated, count) = match self.view(iterable) {
			View::Range {
				start: Some(start),
				end: Some(end),
				inclusive,
			} => {
				let start = self.expr(start)?;
				let start = self.convert(start, Repr::Int(IntRepr::I64), iterable.index())?;
				let end = self.expr(end)?;
				let end = self.convert(end, Repr::Int(IntRepr::I64), iterable.index())?;
				let mut count = self.builder.build_int_sub(end.int(), start.int(), "")?;
				if inclusive {
					count = self
						.builder
						.build_int_add(count, i64.const_int(1, false), "")?;
				}
				(Iterated::Range(start.int()), count)
			}
			_ => {
				let value = self.expr(iterable)?;
				match value.repr {
					Repr::Array(item) => {
						let array = value.value.expect("an array").into_pointer_value();
						let (_, length) = self.array_parts(array)?;
						(Iterated::Array(array, item), length)
					}
					Repr::Growing(item) => {
						let array = value.value.expect("a growing array").into_pointer_value();
						let length = self.growing_length(&self.builder, array)?;
						(Iterated::Growing(array, item), length)
					}
					Repr::Pool(_) | Repr::Columnar(_) => {
						// the addresses of the elements, in slot order: an
						// iteration finds its element in one load
						let place = self.pool_place(value.repr, iterable)?;
						let size = self.pool_size(value)?.int();
						let addresses = self.new_array(Repr::Pointer, i64.const_zero(), size)?;
						let address_size = self.item_size_const(Repr::Pointer);
						let pool = value.value.expect("a pool").into_pointer_value();
						self.pool_each(pool, |this, element, _| {
							let slot = this
								.call_value(
									this.runtime.function("fa_array_push_slot")?,
									&[addresses.into(), address_size.into()],
								)?
								.into_pointer_value();
							this.builder.build_store(slot, element)?;
							Ok(())
						})?;
						self.cleanups
							.borrow_mut()
							.push(super::ownership::Cleanup::Shallow(addresses, address_size));
						(Iterated::Elements(addresses, place), size)
					}
					Repr::Never => return Ok(value),
					_ => {
						return Err(
							self.unsupported(iterable.index(), "`for parallel` over this value")
						);
					}
				}
			}
		};
		let captures = self.parallel_captures(node, body);
		let mut locals = Vec::with_capacity(captures.len());
		for &symbol in &captures {
			locals.push((symbol, self.slot_of(symbol, node)?));
		}
		// the values given with `provide` and the requirements the function
		// was given, which the calls of the body pass on
		let mut provided: Vec<(NodeId, Slot<'ctx>)> = self
			.provided
			.iter()
			.map(|(&key, &slot)| (key, slot))
			.collect();
		provided.sort_by_key(|(key, _)| *key);
		let outer = Outer {
			locals,
			provided,
			requirements: self.frame().requirements.clone(),
		};
		// the environment: the outer places, then the array or the start
		let pointer = self.pointer_type();
		let mut fields: Vec<BasicTypeEnum<'ctx>> =
			(0..outer.len()).map(|_| pointer.into()).collect();
		fields.push(pointer.into());
		fields.push(i64.into());
		let structure = self.context.struct_type(&fields, false);
		let environment = self.alloca_type(structure.into(), "parallel")?;
		let places = outer
			.locals
			.iter()
			.map(|(_, slot)| slot.pointer)
			.chain(outer.provided.iter().map(|(_, slot)| slot.pointer))
			.chain(outer.requirements.iter().map(|(pointer, _)| *pointer));
		for (index, place) in places.enumerate() {
			let field = self
				.builder
				.build_struct_gep(structure, environment, index as u32, "")?;
			self.builder.build_store(field, place)?;
		}
		let extra = outer.len() as u32;
		let (array_value, start_value) = match iterated {
			Iterated::Range(start) => (pointer.const_null(), start),
			Iterated::Array(array, _)
			| Iterated::Growing(array, _)
			| Iterated::Elements(array, _) => (array, i64.const_zero()),
		};
		let field = self
			.builder
			.build_struct_gep(structure, environment, extra, "")?;
		self.builder.build_store(field, array_value)?;
		let field = self
			.builder
			.build_struct_gep(structure, environment, extra + 1, "")?;
		self.builder.build_store(field, start_value)?;
		let items = match iterated {
			Iterated::Range(_) => Items::Range,
			Iterated::Array(_, item) => Items::Array(item),
			Iterated::Growing(_, item) => Items::Growing(item),
			Iterated::Elements(_, place) => Items::Elements(place),
		};
		let function =
			self.outline_parallel_body(node, pattern, has_index, body, items, structure, &outer)?;
		let function_pointer = function.as_global_value().as_pointer_value();
		let error = if self.target == Target::Native {
			let drop_error = self
				.box_drop_function()
				.as_global_value()
				.as_pointer_value();
			self.call_value(
				self.runtime.function("fa_parallel_for")?,
				&[
					count.into(),
					function_pointer.into(),
					environment.into(),
					threads.into(),
					drop_error.into(),
				],
			)?
			.into_pointer_value()
		} else {
			// no threads: the iterations in order
			let failed = self.alloca(Repr::Int(IntRepr::I64), "failed")?;
			let positive =
				self.builder
					.build_int_compare(IntPredicate::SGT, count, i64.const_zero(), "")?;
			let count = self
				.builder
				.build_select(positive, count, i64.const_zero(), "")?
				.into_int_value();
			self.builder
				.build_call(
					function,
					&[
						environment.into(),
						i64.const_zero().into(),
						count.into(),
						failed.pointer.into(),
					],
					"",
				)?
				.try_as_basic_value()
				.basic()
				.expect("an error")
				.into_pointer_value()
		};
		self.raise_if_error(error)?;
		Ok(Val::void())
	}

	/// The body of a parallel loop as a function of a range of iterations.
	#[allow(clippy::too_many_arguments)]
	fn outline_parallel_body(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		body: NodeId,
		items: Items,
		structure: inkwell::types::StructType<'ctx>,
		outer: &Outer<'ctx>,
	) -> Result<FunctionValue<'ctx>, Error> {
		let pointer = self.pointer_type();
		let i64 = self.context.i64_type();
		let function_type = pointer.fn_type(
			&[pointer.into(), i64.into(), i64.into(), pointer.into()],
			false,
		);
		let function =
			self.module
				.add_function("fa.parallel", function_type, Some(Linkage::Internal));
		self.runtime.add_attributes(function, &["nounwind"]);
		// the state of the function around the loop, restored after
		let resume = self.builder.get_insert_block().expect("a block");
		let outer_frame = self.frame.take();
		let owner = outer_frame.as_ref().and_then(|frame| frame.owner);
		let saved_slots = self.slots.clone();
		let saved_provided = self.provided.clone();
		let saved_owned = std::mem::take(&mut self.owned_locals);
		self.begin_function(function, Repr::Void, owner);
		self.frame_mut().throws = true;
		let result = self.parallel_body(
			function, node, pattern, has_index, body, items, structure, outer,
		);
		let ended = self.end_function();
		self.frame = outer_frame;
		self.slots = saved_slots;
		self.provided = saved_provided;
		self.owned_locals = saved_owned;
		self.builder.position_at_end(resume);
		result?;
		ended?;
		Ok(function)
	}

	#[allow(clippy::too_many_arguments)]
	fn parallel_body(
		&mut self,
		function: FunctionValue<'ctx>,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		body: NodeId,
		items: Items,
		structure: inkwell::types::StructType<'ctx>,
		outer: &Outer<'ctx>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let pointer = self.pointer_type();
		let environment = function
			.get_nth_param(0)
			.expect("an environment")
			.into_pointer_value();
		let start = function.get_nth_param(1).expect("a start").into_int_value();
		let end = function.get_nth_param(2).expect("an end").into_int_value();
		let failed = function
			.get_nth_param(3)
			.expect("a failed index")
			.into_pointer_value();
		// the places of the function around it, from the environment
		let mut places = Vec::with_capacity(outer.len());
		for index in 0..outer.len() {
			let field = self
				.builder
				.build_struct_gep(structure, environment, index as u32, "")?;
			places.push(
				self.builder
					.build_load(pointer, field, "")?
					.into_pointer_value(),
			);
		}
		let mut places = places.into_iter();
		for (symbol, slot) in &outer.locals {
			let place = places.next().expect("a local");
			self.slots.insert(
				*symbol,
				Slot {
					pointer: place,
					repr: slot.repr,
				},
			);
		}
		for (key, slot) in &outer.provided {
			let place = places.next().expect("a provided value");
			self.provided.insert(
				*key,
				Slot {
					pointer: place,
					repr: slot.repr,
				},
			);
		}
		let requirements: Vec<(PointerValue<'ctx>, TypeId)> = outer
			.requirements
			.iter()
			.map(|(_, key)| (places.next().expect("a requirement"), *key))
			.collect();
		self.frame_mut().requirements = requirements;
		let extra = outer.len() as u32;
		let field = self
			.builder
			.build_struct_gep(structure, environment, extra, "")?;
		let array = self
			.builder
			.build_load(pointer, field, "array")?
			.into_pointer_value();
		let field = self
			.builder
			.build_struct_gep(structure, environment, extra + 1, "")?;
		let first = self
			.builder
			.build_load(i64, field, "first")?
			.into_int_value();
		let variable = pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern));
		let index_symbol = if has_index {
			self.analysis.symbols.symbol_of_decl(node)
		} else {
			None
		};
		let counter = self.alloca(Repr::Int(IntRepr::I64), "iteration")?;
		self.builder.build_store(counter.pointer, start)?;
		let test = self.append_block("parallel_test");
		let body_block = self.append_block("parallel_body");
		let step = self.append_block("parallel_step");
		let exit = self.append_block("parallel_end");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let position = self.load(counter)?.int();
		let more = self
			.builder
			.build_int_compare(IntPredicate::SLT, position, end, "")?;
		self.builder
			.build_conditional_branch(more, body_block, exit)?;
		self.builder.position_at_end(body_block);
		// an error of this iteration reports its index
		self.builder.build_store(failed, position)?;
		let mark = self.mark();
		let item_slot = match items {
			Items::Range => None,
			Items::Array(item_type) => {
				let item = self.item_repr(item_type, node)?;
				let (items, _) = self.array_parts(array)?;
				let item_pointer = self.item_pointer(items, item, position)?;
				Some(Slot {
					pointer: item_pointer,
					repr: item,
				})
			}
			Items::Growing(item_type) => {
				let item = self.item_repr(item_type, node)?;
				let item_pointer = self.growing_at(&self.builder, array, item, position)?;
				Some(Slot {
					pointer: item_pointer,
					repr: item,
				})
			}
			Items::Elements(place) => {
				let (addresses, _) = self.array_parts(array)?;
				let address = self.item_pointer(addresses, Repr::Pointer, position)?;
				let element = self
					.builder
					.build_load(pointer, address, "element")?
					.into_pointer_value();
				Some(Slot {
					pointer: element,
					repr: place,
				})
			}
		};
		match item_slot {
			None => {
				let value = self.builder.build_int_add(first, position, "")?;
				if let Some(symbol) = variable {
					let slot = self.slot_of(symbol, node)?;
					self.store(slot, Val::new(Repr::Int(IntRepr::I64), value), node)?;
				}
			}
			Some(item_slot) => {
				let copied = variable.is_some_and(|symbol| {
					matches!(
						self.analysis.symbols.symbol(symbol).kind,
						SymbolKind::LoopVariable { copy: true }
					)
				});
				let destructured =
					pattern.and_then(|pattern| self.analysis.pattern(pattern).cloned());
				if let Some(destructured) = destructured {
					self.bind_item_pattern(&destructured, item_slot, node)?;
				} else if let Some(symbol) = variable {
					if copied {
						let copy = self.slot_of(symbol, node)?;
						let value = self.load(item_slot)?;
						let value = self.emit_copy(value)?;
						self.store(copy, value, node)?;
						self.cleanups
							.borrow_mut()
							.push(super::ownership::Cleanup::Local(copy));
					} else {
						self.slots.insert(symbol, item_slot);
					}
				}
			}
		}
		if let Some(symbol) = index_symbol {
			let slot = self.slot_of(symbol, node)?;
			self.store(slot, Val::new(Repr::Int(IntRepr::I64), position), node)?;
		}
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next: step,
			value: None,
			mark,
		});
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.end_scope(mark, None)?;
		self.branch_to(step)?;
		self.builder.position_at_end(step);
		let position = self.load(counter)?.int();
		let next = self
			.builder
			.build_int_nsw_add(position, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(exit);
		self.builder.build_return(Some(&pointer.const_null()))?;
		self.cleanups.borrow_mut().clear();
		Ok(())
	}

	// ── parallel views ───────────────────────────────────────────────────

	/// `collection.parallel().operation(…)` (`Analysis::parallel_operations`):
	/// the callbacks run on the threads of `fa_parallel_for` over the items
	/// of the collection (an array, or a view of it as one), each writing
	/// its own slot of a result; `filter`, `some` and `every` then read the
	/// flags in order, `reduce` combines the partial results of chunks of
	/// items (its function is associative: concurrency.md), `sum` adds in
	/// order.
	pub(crate) fn parallel_operation(
		&mut self,
		node: NodeId,
		operation: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let collection = self.receiver(node)?;
		if let Repr::Map(_) = collection.repr {
			return self.parallel_map_operation(node, operation, collection, info);
		}
		let view = match collection.repr {
			Repr::Never => return Ok(collection),
			Repr::Array(_) => collection,
			Repr::Growing(_) => self.growing_view(collection)?,
			Repr::Pool(_) | Repr::Columnar(_) => self.pool_view(collection, node)?,
			Repr::Set(set) => {
				let TypeKind::Collection(_, item, _) = self.types().kind(set) else {
					return Err(self.unsupported(node.index(), "`parallel()` of this set"));
				};
				self.array_of_table(collection, Repr::Array(item), node)?
			}
			_ => return Err(self.unsupported(node.index(), "`parallel()` of this collection")),
		};
		let arguments: Vec<NodeId> = info
			.arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		let mut values = Vec::with_capacity(arguments.len());
		for &argument in &arguments {
			let value = self.expr(argument)?;
			if value.repr == Repr::Never {
				return Ok(value);
			}
			values.push(value);
		}
		let (array, item) = self.array_of(view, node)?;
		let (_, length) = self.array_parts(array)?;
		let i64 = self.context.i64_type();
		match (operation, values.as_slice()) {
			("map", [callback]) => {
				let Repr::Array(result_type) = self.node_repr(node)? else {
					return Err(self.unsupported(node.index(), "this `map`"));
				};
				let result_item = self.item_repr(result_type, node)?;
				let result = self.zeroed_array(result_item, length)?;
				// dropped with the statement if an iteration fails
				let result_value = self.fresh(Val::new(Repr::Array(result_type), result));
				let callback_node = arguments[0];
				let closure = callback.value.expect("a function");
				let closure_repr = callback.repr;
				self.parallel_each(
					length,
					&[closure, array.into(), result.into()],
					|this, env, index| {
						let callback = Val::new(closure_repr, env[0]);
						let array = env[1].into_pointer_value();
						let result = env[2].into_pointer_value();
						let slot = this.array_item_slot(array, item, index)?;
						let index_value = Val::new(Repr::Int(IntRepr::I64), index);
						let mapped = this.call_collection_callback(
							callback,
							callback_node,
							&[],
							&[slot],
							&[index_value],
						)?;
						let mapped = this.owned_as(mapped, result_item, None)?;
						let target = this.array_item_slot(result, result_item, index)?;
						if let Some(raw) = mapped.value {
							this.builder.build_store(target.pointer, raw)?;
						}
						Ok(())
					},
				)?;
				Ok(result_value)
			}
			("filter" | "some" | "every", [callback]) => {
				let flags = self.parallel_flags(array, item, length, *callback, arguments[0])?;
				let flag_at =
					|this: &mut Self, index: IntValue<'ctx>| -> Result<IntValue<'ctx>, Error> {
						let slot = this.array_item_slot(flags, Repr::Bool, index)?;
						Ok(this.load(slot)?.int())
					};
				if operation == "filter" {
					// the kept items in order, copied as `filter` copies them
					let result = self.empty_array(item, length)?;
					let result_value = Val::new(view.repr, result);
					self.for_each_index(length, |this, index| {
						let keep = flag_at(this, index)?;
						let push = this.append_block("parallel_keep");
						let next = this.append_block("parallel_next");
						this.builder.build_conditional_branch(keep, push, next)?;
						this.builder.position_at_end(push);
						let value = this.load(this.array_item_slot(array, item, index)?)?;
						this.array_push(result_value, value, node, None)?;
						this.builder.build_unconditional_branch(next)?;
						this.builder.position_at_end(next);
						Ok(())
					})?;
					let filtered = self.fresh(result_value);
					// a filter of another collection gives its kind back
					let repr = self.node_repr(node)?;
					return self.collection_of_items(filtered, repr, node);
				}
				// `some`: one set flag; `every`: no clear one
				let found = self.alloca(Repr::Bool, "found")?;
				let every = operation == "every";
				let bool_type = self.context.bool_type();
				self.builder
					.build_store(found.pointer, bool_type.const_int(u64::from(every), false))?;
				self.for_each_index(length, |this, index| {
					let flag = flag_at(this, index)?;
					let current = this.load(found)?.int();
					let next = if every {
						this.builder.build_and(current, flag, "")?
					} else {
						this.builder.build_or(current, flag, "")?
					};
					this.builder.build_store(found.pointer, next)?;
					Ok(())
				})?;
				let found = self.load(found)?;
				Ok(found)
			}
			("reduce", [callback, initial]) => {
				let repr = self.node_repr(node)?;
				let callback_node = arguments[0];
				// chunks of items, each folded on a thread from its first item
				let chunks = i64.const_int(256, false);
				let fewer =
					self.builder
						.build_int_compare(IntPredicate::SLT, length, chunks, "")?;
				let chunks = self
					.builder
					.build_select(fewer, length, chunks, "chunks")?
					.into_int_value();
				let partials = self.zeroed_array(repr, chunks)?;
				let partials_value =
					self.fresh(Val::new(Repr::Array(self.type_of(node)), partials));
				let closure = callback.value.expect("a function");
				let closure_repr = callback.repr;
				self.parallel_each(
					chunks,
					&[
						closure,
						array.into(),
						partials.into(),
						length.into(),
						chunks.into(),
					],
					|this, env, chunk| {
						let callback = Val::new(closure_repr, env[0]);
						let array = env[1].into_pointer_value();
						let partials = env[2].into_pointer_value();
						let length = env[3].into_int_value();
						let chunks = env[4].into_int_value();
						let i64 = this.context.i64_type();
						// items [chunk × length / chunks, (chunk + 1) × length / chunks)
						let first = this.builder.build_int_mul(chunk, length, "")?;
						let first = this.builder.build_int_signed_div(first, chunks, "")?;
						let after =
							this.builder
								.build_int_add(chunk, i64.const_int(1, false), "")?;
						let end = this.builder.build_int_mul(after, length, "")?;
						let end = this.builder.build_int_signed_div(end, chunks, "")?;
						let accumulator = this.alloca(repr, "partial")?;
						let start = this.load(this.array_item_slot(array, item, first)?)?;
						let start = this.owned_as(start, repr, None)?;
						this.store(accumulator, start, node)?;
						let rest = this.builder.build_int_sub(end, first, "")?;
						let rest = this
							.builder
							.build_int_sub(rest, i64.const_int(1, false), "")?;
						this.for_each_index(rest, |this, offset| {
							let index = this.builder.build_int_add(first, offset, "")?;
							let index =
								this.builder
									.build_int_add(index, i64.const_int(1, false), "")?;
							let slot = this.array_item_slot(array, item, index)?;
							let current = this.load(accumulator)?;
							let index_value = Val::new(Repr::Int(IntRepr::I64), index);
							let next = this.call_collection_callback(
								callback,
								callback_node,
								&[current],
								&[slot],
								&[index_value],
							)?;
							let next = this.owned_as(next, repr, None)?;
							this.emit_drop(current)?;
							this.store(accumulator, next, node)?;
							Ok(())
						})?;
						let partial = this.load(accumulator)?;
						let target = this.array_item_slot(partials, repr, chunk)?;
						if let Some(raw) = partial.value {
							this.builder.build_store(target.pointer, raw)?;
						}
						Ok(())
					},
				)?;
				// the partial results, in order, from the initial value
				let combined = self.array_method_more(
					node,
					"Array#reduce",
					partials_value,
					&[*callback, *initial],
					&arguments,
				)?;
				Ok(combined)
			}
			("sum", []) => self.array_method_more(node, "Array#sum", view, &[], &[]),
			_ => Err(self.unsupported(node.index(), &format!("`{operation}` of a parallel view"))),
		}
	}

	/// An operation of a parallel view of a map: its callbacks take a key
	/// and a value, read from arrays of views of the entries (in the map's
	/// order); `map` and `filter` give a map with the same keys in the same
	/// order.
	fn parallel_map_operation(
		&mut self,
		node: NodeId,
		operation: &str,
		map: Val<'ctx>,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let Some(&Argument::Given(callback_node)) = info
			.arguments
			.iter()
			.find(|argument| matches!(argument, Argument::Given(_)))
		else {
			return Err(self.unsupported(node.index(), "this parallel operation of a map"));
		};
		let callback = self.expr(callback_node)?;
		if callback.repr == Repr::Never {
			return Ok(callback);
		}
		let table = self.table_of(map.repr)?;
		let Some(value_repr) = table.value else {
			return Err(self.unsupported(node.index(), "this map"));
		};
		let key_repr = table.key;
		let pointer = map.value.expect("a map").into_pointer_value();
		let length = self.table_size(map)?.int();
		// the keys and the values, as views, in the map's order
		let zero = self.context.i64_type().const_zero();
		let keys = self.new_array(key_repr, zero, length)?;
		let values = self.new_array(value_repr, zero, length)?;
		let key_size = self.item_size_const(key_repr);
		let value_size = self.item_size_const(value_repr);
		let push = self.runtime.function("fa_array_push_slot")?;
		self.for_each_entry(pointer, table, |this, entry, _| {
			for (array, size, is_value) in [(keys, key_size, false), (values, value_size, true)] {
				let loaded = this.load(this.entry_slot(entry, table, is_value)?)?;
				let slot = this
					.call_value(push, &[array.into(), size.into()])?
					.into_pointer_value();
				if let Some(raw) = loaded.value {
					this.builder.build_store(slot, raw)?;
				}
			}
			Ok(())
		})?;
		for (array, size) in [(keys, key_size), (values, value_size)] {
			self.cleanups
				.borrow_mut()
				.push(super::ownership::Cleanup::Shallow(array, size));
		}
		let closure = callback.value.expect("a function");
		let closure_repr = callback.repr;
		let entry_slots =
			|this: &mut Self, keys: PointerValue<'ctx>, values: PointerValue<'ctx>, index| {
				Ok::<_, Error>([
					this.array_item_slot(keys, key_repr, index)?,
					this.array_item_slot(values, value_repr, index)?,
				])
			};
		if operation == "map" {
			let result_repr = self.node_repr(node)?;
			let result_table = self.table_of(result_repr)?;
			let Some(result_item) = result_table.value else {
				return Err(self.unsupported(node.index(), "this `map`"));
			};
			let results = self.zeroed_array(result_item, length)?;
			let item_type = match self.types().kind(self.type_of(node)) {
				TypeKind::Collection(_, item, _) => item,
				_ => return Err(self.unsupported(node.index(), "this `map`")),
			};
			// dropped with the statement (the map keeps copies)
			self.fresh(Val::new(Repr::Array(item_type), results));
			self.parallel_each(
				length,
				&[closure, keys.into(), values.into(), results.into()],
				|this, env, index| {
					let callback = Val::new(closure_repr, env[0]);
					let slots = entry_slots(
						this,
						env[1].into_pointer_value(),
						env[2].into_pointer_value(),
						index,
					)?;
					let mapped =
						this.call_collection_callback(callback, callback_node, &[], &slots, &[])?;
					let mapped = this.owned_as(mapped, result_item, None)?;
					let target =
						this.array_item_slot(env[3].into_pointer_value(), result_item, index)?;
					if let Some(raw) = mapped.value {
						this.builder.build_store(target.pointer, raw)?;
					}
					Ok(())
				},
			)?;
			let result = self.table_new(result_table, length)?;
			self.for_each_index(length, |this, index| {
				let key = this.load(this.array_item_slot(keys, key_repr, index)?)?;
				let key = this.emit_copy(key)?;
				let key = this.convert(key, result_table.key, node.index())?;
				let value = this.load(this.array_item_slot(results, result_item, index)?)?;
				let value = this.emit_copy(value)?;
				this.table_insert(result, result_table, key, Some(value), node)?;
				Ok(())
			})?;
			return Ok(self.fresh(Val::new(result_repr, result)));
		}
		// a flag per entry
		let flags = self.zeroed_array(Repr::Bool, length)?;
		self.fresh(Val::new(Repr::Array(TypeId::BOOLEAN), flags));
		self.parallel_each(
			length,
			&[closure, keys.into(), values.into(), flags.into()],
			|this, env, index| {
				let callback = Val::new(closure_repr, env[0]);
				let slots = entry_slots(
					this,
					env[1].into_pointer_value(),
					env[2].into_pointer_value(),
					index,
				)?;
				let holds =
					this.call_collection_callback(callback, callback_node, &[], &slots, &[])?;
				let holds = this.convert(holds, Repr::Bool, callback_node.index())?;
				let target =
					this.array_item_slot(env[3].into_pointer_value(), Repr::Bool, index)?;
				this.builder
					.build_store(target.pointer, holds.value.expect("a flag"))?;
				Ok(())
			},
		)?;
		match operation {
			"filter" => {
				let result_repr = self.node_repr(node)?;
				let result_table = self.table_of(result_repr)?;
				let result = self.table_new(result_table, zero)?;
				self.for_each_index(length, |this, index| {
					let keep = this
						.load(this.array_item_slot(flags, Repr::Bool, index)?)?
						.int();
					let copy = this.append_block("parallel_keep");
					let next = this.append_block("parallel_next");
					this.builder.build_conditional_branch(keep, copy, next)?;
					this.builder.position_at_end(copy);
					let key = this.load(this.array_item_slot(keys, key_repr, index)?)?;
					let key = this.emit_copy(key)?;
					let value = this.load(this.array_item_slot(values, value_repr, index)?)?;
					let value = this.emit_copy(value)?;
					this.table_insert(result, result_table, key, Some(value), node)?;
					this.builder.build_unconditional_branch(next)?;
					this.builder.position_at_end(next);
					Ok(())
				})?;
				Ok(self.fresh(Val::new(result_repr, result)))
			}
			"some" | "every" => {
				let every = operation == "every";
				let found = self.alloca(Repr::Bool, "found")?;
				self.builder.build_store(
					found.pointer,
					self.context.bool_type().const_int(u64::from(every), false),
				)?;
				self.for_each_index(length, |this, index| {
					let flag = this
						.load(this.array_item_slot(flags, Repr::Bool, index)?)?
						.int();
					let current = this.load(found)?.int();
					let next = if every {
						this.builder.build_and(current, flag, "")?
					} else {
						this.builder.build_or(current, flag, "")?
					};
					this.builder.build_store(found.pointer, next)?;
					Ok(())
				})?;
				self.load(found)
			}
			_ => Err(self.unsupported(
				node.index(),
				&format!("`{operation}` of a parallel view of a map"),
			)),
		}
	}

	/// A collection of copies of the items of an array (dropped with the
	/// statement): the array itself when it is of that kind.
	fn collection_of_items(
		&mut self,
		array: Val<'ctx>,
		repr: Repr,
		node: NodeId,
	) -> Result<Val<'ctx>, Error> {
		if repr == array.repr {
			return Ok(array);
		}
		let (pointer, item) = self.array_of(array, node)?;
		let (_, length) = self.array_parts(pointer)?;
		match repr {
			Repr::Pool(_) | Repr::Columnar(_) => {
				let element = self.pool_element(repr, node)?;
				let place = self.pool_place(repr, node)?;
				let pool = self.collection_new(&self.builder, repr, element)?;
				let add = self.runtime.function("fa_pool_add_slot")?;
				self.for_each_item_value(pointer, item, length, |this, value| {
					let copy = this.emit_copy(value)?;
					let copy = this.convert(copy, element, node.index())?;
					let slot = this.call_value(add, &[pool.into()])?.into_pointer_value();
					this.element_write(&this.builder, place, slot, copy.value.expect("an element"))
				})?;
				Ok(self.fresh(Val::new(repr, pool)))
			}
			Repr::Set(_) => {
				let table = self.table_of(repr)?;
				let set = self.table_new(table, length)?;
				self.for_each_item_value(pointer, item, length, |this, value| {
					let copy = this.emit_copy(value)?;
					let copy = this.convert(copy, table.key, node.index())?;
					this.table_insert(set, table, copy, None, node)?;
					Ok(())
				})?;
				Ok(self.fresh(Val::new(repr, set)))
			}
			_ => self.convert(array, repr, node.index()),
		}
	}

	/// An array of `length` items set to zero (a value whose drop does
	/// nothing until it is written).
	fn zeroed_array(
		&mut self,
		item: Repr,
		length: IntValue<'ctx>,
	) -> Result<PointerValue<'ctx>, Error> {
		let array = self.new_array(item, length, length)?;
		let (items, _) = self.array_parts(array)?;
		let size = self.item_size(item)?;
		let bytes = self.builder.build_int_mul(size, length, "")?;
		self.builder
			.build_memset(items, 8, self.context.i8_type().const_zero(), bytes)?;
		Ok(array)
	}

	/// The results of a predicate on every item, computed in parallel: an
	/// array of `Boolean`s.
	fn parallel_flags(
		&mut self,
		array: PointerValue<'ctx>,
		item: Repr,
		length: IntValue<'ctx>,
		callback: Val<'ctx>,
		callback_node: NodeId,
	) -> Result<PointerValue<'ctx>, Error> {
		let flags = self.zeroed_array(Repr::Bool, length)?;
		self.fresh(Val::new(Repr::Array(TypeId::BOOLEAN), flags));
		let closure = callback.value.expect("a function");
		let closure_repr = callback.repr;
		self.parallel_each(
			length,
			&[closure, array.into(), flags.into()],
			|this, env, index| {
				let callback = Val::new(closure_repr, env[0]);
				let array = env[1].into_pointer_value();
				let flags = env[2].into_pointer_value();
				let slot = this.array_item_slot(array, item, index)?;
				let index_value = Val::new(Repr::Int(IntRepr::I64), index);
				let holds = this.call_collection_callback(
					callback,
					callback_node,
					&[],
					&[slot],
					&[index_value],
				)?;
				let holds = this.convert(holds, Repr::Bool, callback_node.index())?;
				let target = this.array_item_slot(flags, Repr::Bool, index)?;
				this.builder
					.build_store(target.pointer, holds.value.expect("a flag"))?;
				Ok(())
			},
		)?;
		Ok(flags)
	}

	/// Runs `body` for every index in `0..count` on the threads of
	/// `fa_parallel_for` (in order on one thread without threads): an
	/// outlined function reading `values` from an environment, whose errors
	/// are raised here (the one of the smallest failing index).
	fn parallel_each(
		&mut self,
		count: IntValue<'ctx>,
		values: &[BasicValueEnum<'ctx>],
		mut body: impl FnMut(&mut Self, &[BasicValueEnum<'ctx>], IntValue<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let pointer = self.pointer_type();
		let fields: Vec<BasicTypeEnum<'ctx>> =
			values.iter().map(|value| value.get_type()).collect();
		let structure = self.context.struct_type(&fields, false);
		let environment = self.alloca_type(structure.into(), "parallel")?;
		for (index, value) in values.iter().enumerate() {
			let field = self
				.builder
				.build_struct_gep(structure, environment, index as u32, "")?;
			self.builder.build_store(field, *value)?;
		}
		let function_type = pointer.fn_type(
			&[pointer.into(), i64.into(), i64.into(), pointer.into()],
			false,
		);
		let function = self.module.add_function(
			"fa.parallel.operation",
			function_type,
			Some(Linkage::Internal),
		);
		self.runtime.add_attributes(function, &["nounwind"]);
		let resume = self.builder.get_insert_block().expect("a block");
		let outer_frame = self.frame.take();
		let owner = outer_frame.as_ref().and_then(|frame| frame.owner);
		let saved_slots = self.slots.clone();
		let saved_provided = self.provided.clone();
		let saved_owned = std::mem::take(&mut self.owned_locals);
		self.begin_function(function, Repr::Void, owner);
		self.frame_mut().throws = true;
		let result = self.parallel_each_body(function, structure, &fields, &mut body);
		self.cleanups.borrow_mut().clear();
		let ended = self.end_function();
		self.frame = outer_frame;
		self.slots = saved_slots;
		self.provided = saved_provided;
		self.owned_locals = saved_owned;
		self.builder.position_at_end(resume);
		result?;
		ended?;
		let function_pointer = function.as_global_value().as_pointer_value();
		let error = if self.target == Target::Native {
			let drop_error = self
				.box_drop_function()
				.as_global_value()
				.as_pointer_value();
			self.call_value(
				self.runtime.function("fa_parallel_for")?,
				&[
					count.into(),
					function_pointer.into(),
					environment.into(),
					i64.const_zero().into(),
					drop_error.into(),
				],
			)?
			.into_pointer_value()
		} else {
			// no threads: the iterations in order
			let failed = self.alloca(Repr::Int(IntRepr::I64), "failed")?;
			self.builder
				.build_call(
					function,
					&[
						environment.into(),
						i64.const_zero().into(),
						count.into(),
						failed.pointer.into(),
					],
					"",
				)?
				.try_as_basic_value()
				.basic()
				.expect("an error")
				.into_pointer_value()
		};
		self.raise_if_error(error)
	}

	/// The body of the function of `parallel_each`: the iterations of its range.
	fn parallel_each_body(
		&mut self,
		function: FunctionValue<'ctx>,
		structure: inkwell::types::StructType<'ctx>,
		fields: &[BasicTypeEnum<'ctx>],
		body: &mut impl FnMut(&mut Self, &[BasicValueEnum<'ctx>], IntValue<'ctx>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let pointer = self.pointer_type();
		let environment = function
			.get_nth_param(0)
			.expect("an environment")
			.into_pointer_value();
		let start = function.get_nth_param(1).expect("a start").into_int_value();
		let end = function.get_nth_param(2).expect("an end").into_int_value();
		let failed = function
			.get_nth_param(3)
			.expect("a failed index")
			.into_pointer_value();
		let mut values = Vec::with_capacity(fields.len());
		for (index, field_type) in fields.iter().enumerate() {
			let field = self
				.builder
				.build_struct_gep(structure, environment, index as u32, "")?;
			values.push(self.builder.build_load(*field_type, field, "")?);
		}
		let counter = self.alloca(Repr::Int(IntRepr::I64), "iteration")?;
		self.builder.build_store(counter.pointer, start)?;
		let test = self.append_block("parallel_test");
		let body_block = self.append_block("parallel_body");
		let exit = self.append_block("parallel_end");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let position = self.load(counter)?.int();
		let more = self
			.builder
			.build_int_compare(IntPredicate::SLT, position, end, "")?;
		self.builder
			.build_conditional_branch(more, body_block, exit)?;
		self.builder.position_at_end(body_block);
		// an error of this iteration reports its index
		self.builder.build_store(failed, position)?;
		let mark = self.mark();
		body(self, &values, position)?;
		self.end_scope(mark, None)?;
		let position = self.load(counter)?.int();
		let next = self
			.builder
			.build_int_nsw_add(position, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(exit);
		self.builder.build_return(Some(&pointer.const_null()))?;
		Ok(())
	}
}
