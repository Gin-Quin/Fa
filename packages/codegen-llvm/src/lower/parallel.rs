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
	fn drop_at_function(&self, repr: Repr, offset: u64) -> Result<PointerValue<'ctx>, Error> {
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
	fn byte_offset(
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
	fn size_of_repr(&self, repr: Repr) -> Result<IntValue<'ctx>, Error> {
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
		let ty = self.llvm_type(Repr::Optional(scalar)).into_struct_type();
		let pair = self
			.builder
			.build_insert_value(ty.get_undef(), present, 0, "")?;
		let pair = self.builder.build_insert_value(pair, value, 1, "")?;
		Ok(pair.into_struct_value().into())
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
				let variable = !matches!(
					data.kind,
					SymbolKind::Function
						| SymbolKind::Type | SymbolKind::Namespace
						| SymbolKind::TypeParameter
						| SymbolKind::Test
				);
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
					Repr::Never => return Ok(value),
					_ => {
						return Err(self.unsupported(
							iterable.index(),
							"`for parallel` over this value (a range or an array)",
						));
					}
				}
			}
		};
		let captures = self.parallel_captures(node, body);
		let mut capture_slots = Vec::with_capacity(captures.len());
		for &symbol in &captures {
			capture_slots.push((symbol, self.slot_of(symbol, node)?));
		}
		// the environment: the captured places, then the array or the start
		let pointer = self.pointer_type();
		let mut fields: Vec<BasicTypeEnum<'ctx>> =
			captures.iter().map(|_| pointer.into()).collect();
		fields.push(pointer.into());
		fields.push(i64.into());
		let structure = self.context.struct_type(&fields, false);
		let environment = self.alloca_type(structure.into(), "parallel")?;
		for (index, (_, slot)) in capture_slots.iter().enumerate() {
			let field = self
				.builder
				.build_struct_gep(structure, environment, index as u32, "")?;
			self.builder.build_store(field, slot.pointer)?;
		}
		let extra = captures.len() as u32;
		let (array_value, start_value) = match iterated {
			Iterated::Range(start) => (pointer.const_null(), start),
			Iterated::Array(array, _) => (array, i64.const_zero()),
		};
		let field = self
			.builder
			.build_struct_gep(structure, environment, extra, "")?;
		self.builder.build_store(field, array_value)?;
		let field = self
			.builder
			.build_struct_gep(structure, environment, extra + 1, "")?;
		self.builder.build_store(field, start_value)?;
		let item_type = match iterated {
			Iterated::Array(_, item) => Some(item),
			Iterated::Range(_) => None,
		};
		let function = self.outline_parallel_body(
			node,
			pattern,
			has_index,
			body,
			item_type,
			structure,
			&capture_slots,
		)?;
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
		item_type: Option<TypeId>,
		structure: inkwell::types::StructType<'ctx>,
		captures: &[(SymbolId, Slot<'ctx>)],
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
		let saved_owned = std::mem::take(&mut self.owned_locals);
		self.begin_function(function, Repr::Void, owner);
		self.frame_mut().throws = true;
		let result = self.parallel_body(
			function, node, pattern, has_index, body, item_type, structure, captures,
		);
		let ended = self.end_function();
		self.frame = outer_frame;
		self.slots = saved_slots;
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
		item_type: Option<TypeId>,
		structure: inkwell::types::StructType<'ctx>,
		captures: &[(SymbolId, Slot<'ctx>)],
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
		// the places of the function around it
		for (index, (symbol, slot)) in captures.iter().enumerate() {
			let field = self
				.builder
				.build_struct_gep(structure, environment, index as u32, "")?;
			let place = self
				.builder
				.build_load(pointer, field, "")?
				.into_pointer_value();
			self.slots.insert(
				*symbol,
				Slot {
					pointer: place,
					repr: slot.repr,
				},
			);
		}
		let extra = captures.len() as u32;
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
		match item_type {
			None => {
				let value = self.builder.build_int_add(first, position, "")?;
				if let Some(symbol) = variable {
					let slot = self.slot_of(symbol, node)?;
					self.store(slot, Val::new(Repr::Int(IntRepr::I64), value), node)?;
				}
			}
			Some(item_type) => {
				let item = self.item_repr(item_type, node)?;
				let (items, _) = self.array_parts(array)?;
				let item_pointer = self.item_pointer(items, item, position)?;
				let item_slot = Slot {
					pointer: item_pointer,
					repr: item,
				};
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
}
