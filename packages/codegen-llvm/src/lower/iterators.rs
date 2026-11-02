//! Iterators (types brief §5.0.6): `values()`, `keys()` and the lazy
//! methods `filter`, `map`, `firstValues`, fused into the loop of the
//! method that consumes them (`find`, `reduce`, `every`, `some`, `toArray`,
//! a `for` loop). An iterator is never a runtime value: its chain is read
//! back from the syntax ([`Lowering::iterator_plan`]), through the locals
//! that name one (`let evens = numbers.values().filter(…)` lowers to
//! nothing; its chain runs where `evens` is consumed). Each element goes
//! through every stage before the next one is read, so the callbacks run
//! in the order of a lazy iterator (ar-17).

use fa_analyzer::analysis::{Argument, Callee};
use fa_analyzer::ids::NodeId;
use fa_analyzer::symbols::Resolution;
use fa_analyzer::syntax::View;
use fa_analyzer::types::TypeKind;
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;

use crate::error::Error;
use crate::lower::values::{IntRepr, Repr, Scalar, Val};
use crate::lower::{LoopFrame, Lowering, Slot};

/// A stage of an iterator chain, its callback evaluated.
#[derive(Clone, Copy)]
enum Stage<'ctx> {
	Filter(Val<'ctx>, NodeId),
	/// The callback, the representation of its results, and the slot
	/// owning the result of the round when it needs a drop (zeroed at the
	/// start of each round, dropped at its end: a `filter` after the `map`
	/// may skip the code that would take it).
	Map(Val<'ctx>, NodeId, Repr, Option<Slot<'ctx>>),
	/// `firstValues(count)`: the count and the slot counting the values.
	First(Val<'ctx>, Slot<'ctx>),
}

/// An iterator chain: its source collection and its stages.
struct Plan<'ctx> {
	source: Val<'ctx>,
	/// `keys()` of a map.
	keys: bool,
	stages: Vec<Stage<'ctx>>,
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// Whether a node is an iterator.
	pub(crate) fn is_iterator(&self, node: NodeId) -> bool {
		matches!(self.types().kind(self.type_of(node)), TypeKind::Iterator(..))
	}

	/// The representation of the elements of an iterator node.
	fn iterator_element(&self, node: NodeId) -> Result<Repr, Error> {
		match self.types().kind(self.type_of(node)) {
			TypeKind::Iterator(element, _) => self.item_repr(element, node),
			_ => Err(self.unsupported(node.index(), "this iterator")),
		}
	}

	/// The receiver node of a method call `receiver.method(…)`.
	fn receiver_node(&self, call: NodeId) -> Option<NodeId> {
		let View::Call { callee, .. } = self.view(call) else {
			return None;
		};
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		(members.len() == 1).then_some(target)
	}

	/// The chain of an iterator node, its callbacks evaluated in order.
	fn iterator_plan(&mut self, node: NodeId) -> Result<Plan<'ctx>, Error> {
		let mut inner = node;
		while let View::Group(group) = self.view(inner) {
			inner = group;
		}
		// a local naming an iterator: its chain
		if let View::Name(_) = self.view(inner)
			&& let Resolution::Local(symbol) = self.analysis.resolution(inner)
		{
			let declaration = self.analysis.symbols.symbol(symbol).decl_node;
			if let View::Declaration {
				value: Some(value), ..
			} = self.view(declaration)
			{
				return self.iterator_plan(value);
			}
			return Err(self.unsupported(node.index(), "this iterator"));
		}
		let Some(info) = self.analysis.call(inner).cloned() else {
			return Err(self.unsupported(node.index(), "this iterator"));
		};
		let Callee::Builtin(id) = info.callee else {
			return Err(self.unsupported(node.index(), "this iterator"));
		};
		let path = fa_analyzer::builtins::get(id).path;
		let arguments: Vec<NodeId> = info
			.arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		match path {
			"Array#values" | "Set#values" | "Map#values" | "Map#keys" | "Pool#values" => {
				let source = self.receiver(inner)?;
				let source = self.growing_view_if_needed(source)?;
				Ok(Plan {
					source,
					keys: path == "Map#keys",
					stages: Vec::new(),
				})
			}
			"Iterator#filter" | "Iterator#map" | "Iterator#firstValues" => {
				let Some(receiver) = self.receiver_node(inner) else {
					return Err(self.unsupported(node.index(), "this iterator"));
				};
				let mut plan = self.iterator_plan(receiver)?;
				let [argument] = arguments.as_slice() else {
					// `firstValues()`: one value
					if path == "Iterator#firstValues" {
						let one = Val::new(
							Repr::Int(IntRepr::I64),
							self.context.i64_type().const_int(1, false),
						);
						let taken = self.alloca(Repr::Int(IntRepr::I64), "taken")?;
						plan.stages.push(Stage::First(one, taken));
						return Ok(plan);
					}
					return Err(self.unsupported(node.index(), path));
				};
				let value = self.expr(*argument)?;
				let stage = match path {
					"Iterator#filter" => Stage::Filter(value, *argument),
					"Iterator#map" => {
						let repr = self.iterator_element(inner)?;
						let holder = if self.needs_drop(repr) {
							Some(self.alloca(repr, "mapped")?)
						} else {
							None
						};
						Stage::Map(value, *argument, repr, holder)
					}
					_ => {
						let count = self.convert(value, Repr::Int(IntRepr::I64), argument.index())?;
						let taken = self.alloca(Repr::Int(IntRepr::I64), "taken")?;
						Stage::First(count, taken)
					}
				};
				plan.stages.push(stage);
				Ok(plan)
			}
			_ => Err(self.unsupported(node.index(), &format!("`{path}` as an iterator"))),
		}
	}

	/// A growing array or a pool read as an array (a view), anything else as is.
	fn growing_view_if_needed(&mut self, source: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		match source.repr {
			Repr::Growing(_) => self.growing_view(source),
			_ => Ok(source),
		}
	}

	/// The loop over the elements of a source collection: `body` gets the
	/// slot of each element, the block that reads the next one, the exit and
	/// the mark of the loop.
	fn source_loop(
		&mut self,
		source: Val<'ctx>,
		keys: bool,
		node: NodeId,
		mut body: impl FnMut(&mut Self, Slot<'ctx>, BasicBlock<'ctx>, BasicBlock<'ctx>, usize) -> Result<(), Error>,
	) -> Result<(), Error> {
		let i64 = self.context.i64_type();
		let counter = self.alloca(Repr::Int(IntRepr::I64), "iterator_position")?;
		self.builder
			.build_store(counter.pointer, i64.const_zero())?;
		let test = self.append_block("iterate");
		let fetch = self.append_block("iterate_fetch");
		let block = self.append_block("iterate_body");
		let exit = self.append_block("end_iterate");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let position = self.load(counter)?.int();
		let pointer = source.value.expect("a collection").into_pointer_value();
		let one = i64.const_int(1, false);
		let slot = match source.repr {
			Repr::Array(item_type) => {
				let item = self.item_repr(item_type, node)?;
				let (items, length) = self.array_parts(pointer)?;
				let more = self
					.builder
					.build_int_compare(IntPredicate::SLT, position, length, "")?;
				self.builder.build_conditional_branch(more, fetch, exit)?;
				self.builder.position_at_end(fetch);
				let next = self.builder.build_int_nsw_add(position, one, "")?;
				self.builder.build_store(counter.pointer, next)?;
				self.builder.build_unconditional_branch(block)?;
				self.builder.position_at_end(block);
				Slot {
					pointer: self.item_pointer(items, item, position)?,
					repr: item,
				}
			}
			Repr::Map(_) | Repr::Set(_) => {
				let table = self.table_of(source.repr)?;
				let used = self.table_field(pointer, 2)?;
				let more = self
					.builder
					.build_int_compare(IntPredicate::SLT, position, used, "")?;
				self.builder.build_conditional_branch(more, fetch, exit)?;
				self.builder.position_at_end(fetch);
				let next = self.builder.build_int_nsw_add(position, one, "")?;
				self.builder.build_store(counter.pointer, next)?;
				let entry = self.entry_at(pointer, table, position)?;
				let live_field = self.builder.build_struct_gep(table.entry, entry, 1, "")?;
				let live = self.builder.build_load(i64, live_field, "")?.into_int_value();
				let live =
					self.builder
						.build_int_compare(IntPredicate::NE, live, i64.const_zero(), "")?;
				self.builder.build_conditional_branch(live, block, test)?;
				self.builder.position_at_end(block);
				let value = !keys && table.value.is_some();
				self.entry_slot(entry, table, value)?
			}
			Repr::Pool(item_type) => {
				let item = self.item_repr(item_type, node)?;
				let (found, end) = self.pool_seek(&self.builder, pointer, position)?;
				self.builder.build_conditional_branch(end, exit, fetch)?;
				self.builder.position_at_end(fetch);
				let next = self.builder.build_int_add(found, one, "")?;
				self.builder.build_store(counter.pointer, next)?;
				self.builder.build_unconditional_branch(block)?;
				self.builder.position_at_end(block);
				Slot {
					pointer: self.pool_at(&self.builder, pointer, found)?,
					repr: item,
				}
			}
			_ => return Err(self.unsupported(node.index(), "iterating over this value")),
		};
		let mark = self.mark();
		body(self, slot, test, exit, mark)?;
		self.end_scope(mark, None)?;
		self.branch_to(test)?;
		self.builder.position_at_end(exit);
		Ok(())
	}

	/// Runs the stages on an element, then `consume` on what comes out:
	/// the value, and its slot while no `map` replaced it.
	#[allow(clippy::too_many_arguments)]
	fn run_stages(
		&mut self,
		stages: &[Stage<'ctx>],
		value: Val<'ctx>,
		place: Option<Slot<'ctx>>,
		exit: BasicBlock<'ctx>,
		mark: usize,
		consume: &mut dyn FnMut(&mut Self, Val<'ctx>, Option<Slot<'ctx>>) -> Result<(), Error>,
	) -> Result<(), Error> {
		let Some((stage, rest)) = stages.split_first() else {
			return consume(self, value, place);
		};
		match *stage {
			Stage::Filter(callback, callback_node) => {
				let ty = self.type_of(callback_node);
				let holds = self.call_closure(callback, ty, &[value], callback_node)?;
				let holds = self.convert(holds, Repr::Bool, callback_node.index())?.int();
				let then = self.append_block("iterator_kept");
				let join = self.append_block("iterator_next");
				self.builder.build_conditional_branch(holds, then, join)?;
				self.builder.position_at_end(then);
				self.region(None, callback_node, |this| {
					this.run_stages(rest, value, place, exit, mark, consume)?;
					Ok(Val::void())
				})?;
				self.branch_to(join)?;
				self.builder.position_at_end(join);
				Ok(())
			}
			Stage::Map(callback, callback_node, repr, holder) => {
				let ty = self.type_of(callback_node);
				let mapped = self.call_closure(callback, ty, &[value], callback_node)?;
				let mapped = match holder {
					Some(holder) => {
						let owned = self.owned_as(mapped, repr, Some(callback_node))?;
						self.store(holder, owned, callback_node)?;
						self.load(holder)?
					}
					None => self.convert(mapped, repr, callback_node.index())?,
				};
				self.run_stages(rest, mapped, None, exit, mark, consume)
			}
			Stage::First(count, taken) => {
				let count = count.int();
				let current = self.load(taken)?.int();
				let full =
					self.builder
						.build_int_compare(IntPredicate::SGE, current, count, "")?;
				let stop = self.append_block("iterator_full");
				let go = self.append_block("iterator_take");
				self.builder.build_conditional_branch(full, stop, go)?;
				self.builder.position_at_end(stop);
				self.drop_to(mark)?;
				self.builder.build_unconditional_branch(exit)?;
				self.builder.position_at_end(go);
				let next = self.builder.build_int_nsw_add(
					current,
					current.get_type().const_int(1, false),
					"",
				)?;
				self.builder.build_store(taken.pointer, next)?;
				self.run_stages(rest, value, place, exit, mark, consume)?;
				// the last value taken: the source is not read again
				if self.builder.get_insert_block().is_some_and(|block| block.get_terminator().is_none()) {
					let done =
						self.builder
							.build_int_compare(IntPredicate::SGE, next, count, "")?;
					let leave = self.append_block("iterator_done");
					let more = self.append_block("iterator_more");
					self.builder.build_conditional_branch(done, leave, more)?;
					self.builder.position_at_end(leave);
					self.drop_to(mark)?;
					self.builder.build_unconditional_branch(exit)?;
					self.builder.position_at_end(more);
				}
				Ok(())
			}
		}
	}

	/// Runs an iterator chain: `consume` gets each value that comes out, its
	/// slot while no `map` replaced it, and the exit (to stop early).
	fn run_iterator(
		&mut self,
		node: NodeId,
		mut consume: impl FnMut(
			&mut Self,
			Val<'ctx>,
			Option<Slot<'ctx>>,
			BasicBlock<'ctx>,
			BasicBlock<'ctx>,
			usize,
		) -> Result<(), Error>,
	) -> Result<(), Error> {
		let plan = self.iterator_plan(node)?;
		for stage in &plan.stages {
			if let Stage::First(_, taken) = stage {
				self.builder
					.build_store(taken.pointer, self.context.i64_type().const_zero())?;
			}
		}
		let stages = plan.stages.clone();
		self.source_loop(plan.source, plan.keys, node, |this, slot, next, exit, mark| {
			for stage in &stages {
				if let Stage::Map(_, _, repr, Some(holder)) = *stage {
					this.builder
						.build_store(holder.pointer, super::zero_of(this.llvm_type(repr)))?;
					this.cleanups
						.borrow_mut()
						.push(super::ownership::Cleanup::Local(holder));
				}
			}
			let value = this.load(slot)?;
			let mut consume_here =
				|this: &mut Self, value: Val<'ctx>, place: Option<Slot<'ctx>>| {
					consume(this, value, place, next, exit, mark)
				};
			this.run_stages(&stages, value, Some(slot), exit, mark, &mut consume_here)
		})
	}

	/// The methods that consume an iterator.
	pub(crate) fn iterator_method(
		&mut self,
		node: NodeId,
		path: &str,
		info: &fa_analyzer::analysis::CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let Some(receiver) = self.receiver_node(node) else {
			return Err(self.unsupported(node.index(), path));
		};
		let arguments: Vec<NodeId> = info
			.arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect();
		let element = self.iterator_element(receiver)?;
		match (path, arguments.as_slice()) {
			("Iterator#toArray", []) => {
				let Repr::Array(item_type) = self.node_repr(node)? else {
					return Err(self.unsupported(node.index(), path));
				};
				let item = self.item_repr(item_type, node)?;
				let zero = self.context.i64_type().const_zero();
				let array = self.new_array(item, zero, zero)?;
				let result = Val::new(Repr::Array(item_type), array);
				self.run_iterator(receiver, |this, value, _, _, _, _| {
					this.array_push(result, value, node, None)
				})?;
				Ok(self.fresh(result))
			}
			("Iterator#find", [callback]) => {
				let predicate = self.expr(*callback)?;
				let Some(scalar) = Scalar::of(element) else {
					return Err(self.unsupported(node.index(), path));
				};
				let repr = Repr::Optional(scalar);
				let result = self.alloca(repr, "found")?;
				self.builder
					.build_store(result.pointer, super::zero_of(self.llvm_type(repr)))?;
				let ty = self.type_of(*callback);
				let callback = *callback;
				self.run_iterator(receiver, |this, value, _, _, exit, mark| {
					let holds = this.call_closure(predicate, ty, &[value], callback)?;
					let holds = this.convert(holds, Repr::Bool, callback.index())?.int();
					let found = this.append_block("iterator_found");
					let next = this.append_block("iterator_find_next");
					this.builder.build_conditional_branch(holds, found, next)?;
					this.builder.position_at_end(found);
					let owned = this.owned_as(value, repr, None)?;
					this.store(result, owned, node)?;
					this.drop_to(mark)?;
					this.builder.build_unconditional_branch(exit)?;
					this.builder.position_at_end(next);
					Ok(())
				})?;
				let found = self.load(result)?;
				Ok(self.fresh(found))
			}
			("Iterator#some" | "Iterator#every", [callback]) => {
				let predicate = self.expr(*callback)?;
				let every = path == "Iterator#every";
				let result = self.alloca(Repr::Bool, "answer")?;
				self.builder.build_store(
					result.pointer,
					self.context.bool_type().const_int(every as u64, false),
				)?;
				let ty = self.type_of(*callback);
				let callback = *callback;
				self.run_iterator(receiver, |this, value, _, _, exit, mark| {
					let holds = this.call_closure(predicate, ty, &[value], callback)?;
					let holds = this.convert(holds, Repr::Bool, callback.index())?.int();
					let decided = if every {
						this.builder.build_not(holds, "")?
					} else {
						holds
					};
					let stop = this.append_block("iterator_decided");
					let next = this.append_block("iterator_undecided");
					this.builder.build_conditional_branch(decided, stop, next)?;
					this.builder.position_at_end(stop);
					this.builder.build_store(
						result.pointer,
						this.context.bool_type().const_int(!every as u64, false),
					)?;
					this.drop_to(mark)?;
					this.builder.build_unconditional_branch(exit)?;
					this.builder.position_at_end(next);
					Ok(())
				})?;
				self.load(result)
			}
			("Iterator#reduce", [callback, initial]) => {
				let reducer = self.expr(*callback)?;
				let repr = self.node_repr(node)?;
				let start = self.expr(*initial)?;
				let start = self.owned_as(start, repr, Some(*initial))?;
				let accumulator = self.alloca(repr, "accumulator")?;
				self.store(accumulator, start, node)?;
				let ty = self.type_of(*callback);
				let callback = *callback;
				self.run_iterator(receiver, |this, value, _, _, _, _| {
					let current = this.load(accumulator)?;
					let next = this.call_closure(reducer, ty, &[current, value], callback)?;
					let next = this.owned_as(next, repr, None)?;
					this.emit_drop(current)?;
					this.store(accumulator, next, node)
				})?;
				let result = self.load(accumulator)?;
				Ok(self.fresh(result))
			}
			_ => Err(self.unsupported(node.index(), &format!("`{path}`"))),
		}
	}

	/// `for value in iterator`: the variable designates the element while
	/// no `map` replaced it, else holds the mapped value.
	pub(crate) fn for_iterator(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		iterable: NodeId,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let variable = pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern));
		self.run_iterator(iterable, |this, value, place, next, exit, mark| {
			let inner = this.mark();
			if let Some(symbol) = variable {
				match place {
					Some(slot) => {
						this.slots.insert(symbol, slot);
					}
					None => {
						let slot = this.slot_of(symbol, node)?;
						let owned = this.owned_as(value, slot.repr, None)?;
						this.store(slot, owned, node)?;
						if this.needs_drop(slot.repr) {
							this.cleanups
								.borrow_mut()
								.push(super::ownership::Cleanup::Local(slot));
						}
					}
				}
			}
			// `continue` drops what the round holds and reads the next element
			this.frame_mut().loops.push(LoopFrame {
				exit,
				next,
				value: None,
				mark,
			});
			this.block(body, false)?;
			this.frame_mut().loops.pop();
			this.end_scope(inner, None)?;
			Ok(())
		})?;
		Ok(Val::void())
	}
}
