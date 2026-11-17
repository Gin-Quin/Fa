//! A private immutable nonempty split consumed only by loops and length reads.
//! Its source already satisfies the ordinary split-view lifetime proof. No array
//! value escapes: unsupported observations retain the ordinary representation.

use fa_analyzer::ids::{FileId, NodeId, SymbolId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::View;
use inkwell::IntPredicate;
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use rustc_hash::FxHashMap;

use super::values::{IntRepr, Repr, Val};
use super::{LoopFrame, Lowering, Slot};
use crate::error::Error;

#[derive(Clone, Copy)]
pub(crate) struct SplitStream<'ctx> {
	bytes: PointerValue<'ctx>,
	length: IntValue<'ctx>,
	capacity: IntValue<'ctx>,
	separator: PointerValue<'ctx>,
	separator_length: IntValue<'ctx>,
	count: Slot<'ctx>,
}

pub(crate) type Streams<'ctx> =
	FxHashMap<(FileId, SymbolId, FunctionValue<'ctx>), SplitStream<'ctx>>;

impl<'ctx> Lowering<'_, 'ctx> {
	fn stream_loop_is_readonly(&self, node: NodeId, pattern: Option<NodeId>) -> bool {
		let Some(pattern) = pattern else {
			return true;
		};
		if !matches!(self.view(pattern), View::Name(_)) {
			return false;
		}
		let Some(variable) = self.analysis.symbols.symbol_of_decl(pattern) else {
			return false;
		};
		if !matches!(
			self.analysis.symbols.symbol(variable).kind,
			SymbolKind::LoopVariable { copy: false }
		) || self.referenced_locals().contains(&variable)
		{
			return false;
		}
		let parents = self.parents();
		let home = self.body_of(node.index(), &parents);
		self.uses_of(variable).into_iter().all(|read| {
			read == pattern
				|| (self.body_of(read.index(), &parents) == home
					&& !self.writes_local(read, &parents))
		})
	}

	fn stream_traversal_is_once(
		&self,
		declaration: NodeId,
		traversal: NodeId,
		parents: &[usize],
	) -> bool {
		let mut scope = declaration.index();
		loop {
			let Some(&parent) = parents.get(scope) else {
				return false;
			};
			if parent == usize::MAX {
				return false;
			}
			scope = parent;
			if matches!(
				self.view(NodeId::new(scope)),
				View::Block { .. } | View::Module { .. }
			) {
				break;
			}
		}
		let mut current = traversal.index();
		loop {
			let Some(&parent) = parents.get(current) else {
				return false;
			};
			if parent == scope {
				return true;
			}
			if parent == usize::MAX
				|| matches!(
					self.view(NodeId::new(parent)),
					View::For { .. }
						| View::While { .. }
						| View::WhileLet { .. }
						| View::Loop(_) | View::Function { .. }
						| View::Lambda { .. }
						| View::Start { .. }
						| View::Test { .. }
						| View::Watch { .. }
				) {
				return false;
			}
			current = parent;
		}
	}

	fn can_stream_split(
		&self,
		declaration: NodeId,
		holder: SymbolId,
		value: NodeId,
	) -> Option<NodeId> {
		if self.tasks.asynchronous
			|| self.c_library.is_some()
			|| !self.bindings.borrow().is_empty()
			|| self.referenced_locals().contains(&holder)
		{
			return None;
		}
		let mut value = value;
		while let View::Group(inner) = self.view(value) {
			value = inner;
		}
		let split = self.nonempty_split_filter(value)?;
		if self.split_view_holder(split) != Some(holder) {
			return None;
		}
		let [fa_analyzer::analysis::Argument::Given(separator)] =
			self.analysis.call(split)?.arguments.as_slice()
		else {
			return None;
		};
		if !matches!(self.view(*separator), View::String(text) if !text.is_empty()) {
			return None;
		}
		// Only an actual candidate pays for the opaque-body scan.
		if self.tree.nodes.iter().enumerate().any(|(index, _)| {
			matches!(
				self.view(NodeId::new(index)),
				View::For {
					compile_time: true,
					..
				}
			)
		}) {
			return None;
		}
		let parents = self.parents();
		let mut ancestor = declaration.index();
		while let Some(&parent) = parents.get(ancestor) {
			if parent == usize::MAX {
				break;
			}
			match self.view(NodeId::new(parent)) {
				View::ExportValue { .. } => return None,
				View::Module { .. } | View::Block { .. } => break,
				_ => ancestor = parent,
			}
		}
		let home = self.body_of(declaration.index(), &parents);
		let mut traversal = None;
		for read in self.uses_of(holder) {
			if read == declaration {
				continue;
			}
			if self.body_of(read.index(), &parents) != home {
				return None;
			}
			let mut current = read;
			loop {
				let parent = *parents.get(current.index())?;
				if parent == usize::MAX {
					return None;
				}
				match self.view(NodeId::new(parent)) {
					View::Group(_) => current = NodeId::new(parent),
					View::Access { target, members } if target == current && members.len() == 1 => {
						let member = members.first()?;
						if !matches!(self.analysis.resolution(member), Resolution::Builtin(id)
							if fa_analyzer::builtins::get(id).path == "Array#length")
						{
							return None;
						}
						break;
					}
					View::For {
						iterable,
						pattern,
						compile_time: false,
						..
					} if iterable == current => {
						let this = NodeId::new(parent);
						if traversal.is_some_and(|other| other != this)
							|| !self.stream_traversal_is_once(declaration, this, &parents)
							|| !self.stream_loop_is_readonly(this, pattern)
						{
							return None;
						}
						traversal = Some(this);
						break;
					}
					_ => return None,
				}
			}
		}
		Some(split)
	}

	pub(super) fn declare_split_stream(
		&mut self,
		node: NodeId,
		holder: SymbolId,
		value: NodeId,
	) -> Result<bool, Error> {
		let Some(split) = self.can_stream_split(node, holder, value) else {
			return Ok(false);
		};
		let source = self.receiver(split)?;
		let (bytes, length) = self.string_parts(source)?;
		let capacity = self.string_capacity(source)?;
		let separator = self
			.analysis
			.call(split)
			.expect("a split call")
			.arguments
			.iter()
			.find_map(|argument| {
				if let fa_analyzer::analysis::Argument::Given(node) = argument {
					Some(*node)
				} else {
					None
				}
			})
			.expect("a separator");
		let separator = self.expr(separator)?;
		let (separator, separator_length) = self.string_parts(separator)?;
		let count = self.alloca(Repr::Int(IntRepr::I64), "split_count")?;
		self.builder
			.build_store(count.pointer, self.context.i64_type().const_all_ones())?;
		self.split_streams.insert(
			(self.file, holder, self.frame().function),
			SplitStream {
				bytes,
				length,
				capacity,
				separator,
				separator_length,
				count,
			},
		);
		Ok(true)
	}

	fn split_stream(&self, mut node: NodeId) -> Option<SplitStream<'ctx>> {
		while let View::Group(inner) = self.view(node) {
			node = inner;
		}
		let Resolution::Local(symbol) = self.analysis.resolution(node) else {
			return None;
		};
		self.split_streams
			.get(&(self.file, symbol, self.frame().function))
			.copied()
	}

	pub(super) fn split_stream_length(&mut self, node: NodeId) -> Result<Option<Val<'ctx>>, Error> {
		let Some(stream) = self.split_stream(node) else {
			return Ok(None);
		};
		let count = self.load(stream.count)?.int();
		let known = self.builder.build_int_compare(
			IntPredicate::SGE,
			count,
			self.context.i64_type().const_zero(),
			"",
		)?;
		let calculate = self.append_block("count_split");
		let end = self.append_block("split_counted");
		self.builder
			.build_conditional_branch(known, end, calculate)?;
		self.builder.position_at_end(calculate);
		let counter = self.runtime.function("fa_string_split_count")?;
		let count = self
			.call_value(
				counter,
				&[
					stream.bytes.into(),
					stream.length.into(),
					stream.capacity.into(),
					stream.separator.into(),
					stream.separator_length.into(),
				],
			)?
			.into_int_value();
		let count =
			self.builder
				.build_int_z_extend_or_bit_cast(count, self.context.i64_type(), "")?;
		self.builder.build_store(stream.count.pointer, count)?;
		self.builder.build_unconditional_branch(end)?;
		self.builder.position_at_end(end);
		Ok(Some(self.load(stream.count)?))
	}

	pub(super) fn for_split_stream(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		body: NodeId,
	) -> Result<Option<Val<'ctx>>, Error> {
		let Some(stream) = self.split_stream(iterable) else {
			return Ok(None);
		};
		self.split_stream_path(node, pattern, has_index, stream, body)
			.map(Some)
	}

	fn split_stream_path(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		stream: SplitStream<'ctx>,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let state_type = self.usize.array_type(1);
		let cursor = self.alloca_type(state_type.into(), "split_cursor")?;
		self.builder.build_store(cursor, state_type.const_zero())?;
		let part = self.alloca(Repr::Str, "split_part")?;
		let position = self.alloca(Repr::Int(IntRepr::I64), "split_position")?;
		self.builder
			.build_store(position.pointer, self.context.i64_type().const_zero())?;
		let index_slot = if has_index {
			self.analysis
				.symbols
				.symbol_of_decl(node)
				.map(|symbol| self.slot_of(symbol, node))
				.transpose()?
		} else {
			None
		};
		if let Some(variable) =
			pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern))
		{
			self.slots.insert(variable, part);
		}
		let test = self.append_block("for_split");
		let block = self.append_block("for_split_body");
		let step = self.append_block("for_split_step");
		let finish = self.append_block("split_complete");
		let exit = self.append_block("end_for_split");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let next = self.runtime.function("fa_string_split_next")?;
		let invocation = self.builder.build_call(
			next,
			&[
				stream.bytes.into(),
				stream.length.into(),
				stream.capacity.into(),
				stream.separator.into(),
				stream.separator_length.into(),
				cursor.into(),
				part.pointer.into(),
			],
			"",
		)?;
		// Every separator is a nonempty, well-formed literal; it cannot cut
		// a surrogate pair. Other view constructors retain their capability.
		invocation.add_attribute(
			inkwell::attributes::AttributeLoc::Function,
			self.context
				.create_string_attribute(super::CANONICAL_SPLIT, ""),
		);
		let found = invocation
			.try_as_basic_value()
			.basic()
			.expect("a result")
			.into_int_value();
		let more = self.builder.build_int_compare(
			IntPredicate::NE,
			found,
			found.get_type().const_zero(),
			"",
		)?;
		self.builder.build_conditional_branch(more, block, finish)?;
		self.builder.position_at_end(block);
		self.assume_external_string(part.pointer)?;
		if let Some(index) = index_slot {
			self.store(index, self.load(position)?, node)?;
		}
		let mark = self.mark();
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next: step,
			value: None,
			mark,
		});
		self.coop_point()?;
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.end_scope(mark, None)?;
		self.branch_to(step)?;
		self.builder.position_at_end(step);
		let count = self.load(position)?.int();
		let count = self.builder.build_int_nsw_add(
			count,
			self.context.i64_type().const_int(1, false),
			"",
		)?;
		self.builder.build_store(position.pointer, count)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(finish);
		let count = self.load(position)?.int();
		self.builder.build_store(stream.count.pointer, count)?;
		self.builder.build_unconditional_branch(exit)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}
}
