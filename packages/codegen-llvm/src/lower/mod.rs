//! The lowering of an analyzed Fa file to an LLVM module.
//!
//! One walk over the parser's tree, through the analyzer's view layer
//! (`fa_analyzer::syntax`), reading the side tables of the [`Analysis`]:
//! the type of every node, the resolution of every name, the check bits
//! (`OVERFLOW_NATIVE`, `SINK_CHECK`, `DIVISOR_CHECK`, `MIN_DIV_GUARD`,
//! `CONVERSION_CHECK`), the arithmetic facts, the coercions at sinks, the
//! functions and the calls in parameter order (design/analyzer-design.md
//! §2.6, §3.6).
//!
//! - Every local is an `alloca` in the entry block of its function, which
//!   `mem2reg` turns into registers. A local of the top level that a function
//!   reads is an internal global instead.
//! - Every function is `internal fastcc nounwind`; a parameter with a
//!   default value gets an `i1` telling whether it was given, and the callee
//!   evaluates the default (functions brief: defaults are evaluated in the
//!   callee, after the previous parameters).
//! - The top level of the file is `main` (`_start` on wasm).
//! - Values with a type are [`Val`]s; constructs whose value is used
//!   (`if`, `loop`, `do`) store it in a slot.

mod aliasing;
mod appends;
mod arith;
mod arrays;
mod calls;
mod capacity;
mod chunks;
mod closures;
mod columns;
mod constants;
mod division;
mod foreign;
mod growing;
mod inline_unions;
mod inlining;
mod io;
mod iterators;
mod numbers;
mod objects;
mod ownership;
mod parallel;
mod patterns;
mod pools;
pub(crate) mod program;
mod reactivity;
mod references;
mod relations;
mod tables;
mod tasks;
mod testing;
mod text;
mod trees;
mod tuples;
pub(crate) mod units;
pub(crate) mod values;
mod views;

use fa_analyzer::analysis::{Analysis, FnInfo, NodeFlags};
use fa_analyzer::ids::{FileId, NodeId, ScopeId, SymbolId, TypeId};
use fa_analyzer::session::Session;
use fa_analyzer::symbols::{Resolution, ScopeKind, SymbolKind};
use fa_analyzer::syntax::{self, View};
use fa_analyzer::types::TypeStore;
use fa_parser::TypedSyntaxTree;
use inkwell::attributes::AttributeLoc;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, IntType, StructType};
use inkwell::values::{BasicMetadataValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};
use rustc_hash::FxHashMap;

use crate::error::Error;
use crate::runtime::Runtime;
use crate::target::Target;
use values::{IntRepr, Repr, Val};

/// LLVM's number for the `fastcc` calling convention.
pub(crate) const FAST_CALLING_CONVENTION: u32 = 8;

/// A lowered function declaration.
#[derive(Clone)]
pub(crate) struct Function<'ctx> {
	value: FunctionValue<'ctx>,
	/// The body of an arena builder, called directly inside another builder.
	arena_body: Option<FunctionValue<'ctx>>,
	info: FnInfo,
	/// The representation of each parameter, and whether it has a default
	/// (then an `i1` follows it).
	params: Vec<(Repr, bool)>,
	/// The parameters passed by pointer: the roots of the function's
	/// `mutates` and `invalidates` paths, which it changes in place.
	by_pointer: Vec<bool>,
	/// Read-only aggregate parameters borrowed from the caller's storage.
	borrowed: Vec<bool>,
	ret: Repr,
	/// Whether it throws: it returns `{ result, ptr error }` (or the error alone).
	throws: bool,
	/// A lambda, or a local function using locals of the function around it:
	/// its first parameter is its environment, holding these captures.
	lambda: bool,
	/// The environment holds pointers to the captured locals (a closure that
	/// doesn't escape sees the variables themselves), else copies of them.
	by_reference: bool,
	captures: Vec<SymbolId>,
	/// The function or lambda node (the key of an instance is another).
	declaration: NodeId,
	/// The types of the generic parameters of an instance.
	bindings: FxHashMap<fa_analyzer::ids::ParamId, Repr>,
}

/// The storage of a local: a pointer (an alloca or a global) and its representation.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Slot<'ctx> {
	pointer: PointerValue<'ctx>,
	repr: Repr,
}

/// A loop being lowered: where `break` and `continue` go.
struct LoopFrame<'ctx> {
	exit: BasicBlock<'ctx>,
	next: BasicBlock<'ctx>,
	/// The slot of the value of a `loop` used as a value.
	value: Option<Slot<'ctx>>,
	/// The depth of the cleanup stack around the loop: what `break` and
	/// `continue` drop.
	mark: usize,
}

/// The function being lowered.
struct Frame<'ctx> {
	function: FunctionValue<'ctx>,
	/// Recursive nodes allocated by this function belong to the caller's region.
	arena: bool,
	/// Key comparison applies SameValueZero recursively to composite values.
	same_value_zero: bool,
	/// The block holding the allocas, before `start`.
	allocas: BasicBlock<'ctx>,
	/// The return representation (`Void` for `main`).
	ret: Repr,
	/// The function declaration, `None` for the top level.
	owner: Option<NodeId>,
	loops: Vec<LoopFrame<'ctx>>,
	/// The handlers of the enclosing `catch`es, innermost last, the slot of
	/// their error, and the depth of the cleanup stack around them.
	catches: Vec<(BasicBlock<'ctx>, PointerValue<'ctx>, usize)>,
	/// Whether the function throws (returns its errors).
	throws: bool,
	/// Its requirements (hidden parameters, pointers to what provides
	/// them) and their key types, in the order of `FnSig::requires`.
	requirements: Vec<(PointerValue<'ctx>, TypeId)>,
	/// A coroutine (an async instance), or how `start` runs (`tasks.rs`).
	task: tasks::FrameTask<'ctx>,
}

/// Lowers the analyzed files of a program into one LLVM module. The fields
/// of the file being lowered are swapped with the [`program::FileState`]
/// of another file when the lowering moves to it ([`Lowering::switch_file`]).
pub(crate) struct Lowering<'a, 'ctx> {
	session: &'a Session,
	/// The file being lowered, whether it is a module (its values are
	/// initialized in `Program::init_order`), and the other files' states.
	file: FileId,
	module_file: bool,
	other_files: FxHashMap<FileId, program::FileState<'a, 'ctx>>,
	/// The program's initialization order and entry (`lower_program`).
	init_order: Vec<(FileId, NodeId)>,
	entry: FileId,
	/// The function symbols of the file: their key in `functions` (or
	/// `generic_functions`).
	symbol_functions: FxHashMap<SymbolId, NodeId>,
	/// The exit code of the entry function, set at the end of `main`.
	exit_code: Option<PointerValue<'ctx>>,
	tree: &'a TypedSyntaxTree,
	analysis: &'a Analysis,
	/// The path printed in stop locations.
	path: String,
	context: &'ctx Context,
	module: &'a Module<'ctx>,
	builder: Builder<'ctx>,
	runtime: Runtime<'ctx, 'a>,
	target: Target,
	usize: IntType<'ctx>,
	/// `{ ptr, usize }`: strings and arrays.
	string_type: StructType<'ctx>,
	/// The function declarations, by their `function` node.
	functions: FxHashMap<NodeId, Function<'ctx>>,
	/// The storage of every local reached so far.
	slots: FxHashMap<SymbolId, Slot<'ctx>>,
	/// The function declaration owning each scope (`None`: the top level).
	scope_owners: FxHashMap<ScopeId, Option<NodeId>>,
	/// The top-level locals that functions read: globals.
	shared: FxHashMap<SymbolId, ()>,
	/// The references that may designate nothing (`use found = pool.find(…)`):
	/// their address may be null, and they read as optionals.
	nullable_references: FxHashMap<SymbolId, ()>,
	/// The `push` calls of a counted loop whose array reserved room for
	/// every round first (`arrays.rs`, `reserve_for_pushes`): no capacity check.
	reserved_pushes: FxHashMap<NodeId, ()>,
	/// The `text += character` of a counted loop with room reserved before
	/// it (`appends.rs`): the loop's counter slot, its end and whether it is
	/// inclusive, to keep the room for the rounds left after a longer append.
	reserved_appends: FxHashMap<NodeId, (PointerValue<'ctx>, IntValue<'ctx>, bool)>,
	frame: Option<Frame<'ctx>>,
	/// The constants already emitted: string literals and check sites.
	constants: FxHashMap<Vec<u8>, PointerValue<'ctx>>,
	/// Line starts of the source, for locations.
	lines: Vec<usize>,
	/// The structs of object types.
	object_types: std::cell::RefCell<FxHashMap<TypeId, StructType<'ctx>>>,
	/// The global id of every member of a named union built or tested.
	union_ids: std::cell::RefCell<FxHashMap<(fa_analyzer::ids::NominalId, u16), u64>>,
	/// The generic functions, declared per instance.
	generic_functions: FxHashMap<NodeId, FnInfo>,
	/// The instances of generic functions: the key of their `Function`.
	instances: FxHashMap<(NodeId, Vec<Repr>), NodeId>,
	/// The instances still to lower, with their file.
	pending_instances: Vec<(FileId, NodeId)>,
	/// The representations of the generic parameters of the function being
	/// lowered (an instance).
	bindings: std::cell::RefCell<FxHashMap<fa_analyzer::ids::ParamId, Repr>>,
	/// Whether integer overflows are checked (`Options::overflow_checks`).
	overflow_checks: bool,
	/// The slots of the top-level locals that functions read (globals).
	global_slots: FxHashMap<SymbolId, Slot<'ctx>>,
	/// The globals of module-level `let` values computed at run time: set
	/// once before any function reads them, so a function's loads of them
	/// are `!invariant.load` (not reloaded after calls and stores).
	invariant_globals: rustc_hash::FxHashSet<PointerValue<'ctx>>,
	/// The global id of every member type of an anonymous union.
	type_ids: std::cell::RefCell<FxHashMap<TypeId, u64>>,
	/// The `mutable use` references: the slot holding the address of the
	/// place they designate, and its representation.
	references: FxHashMap<SymbolId, (PointerValue<'ctx>, Repr)>,
	/// The places of the `provide` statements lowered so far.
	provided: FxHashMap<NodeId, Slot<'ctx>>,
	/// The receiver of the method call being lowered when it is already
	/// evaluated (`value?.method()`).
	pending_receiver: Option<Val<'ctx>>,
	/// The owned locals and the temporaries of the function being lowered
	/// (`ownership.rs`).
	cleanups: std::cell::RefCell<Vec<ownership::Cleanup<'ctx>>>,
	/// The compile-time results of the `@for` items being lowered,
	/// innermost last (`Analysis::unrolled_items`, constants.rs).
	item_results: Vec<fa_analyzer::analysis::ItemResults>,
	/// The cleanup stacks of the functions whose lowering is suspended.
	saved_cleanups: Vec<Vec<ownership::Cleanup<'ctx>>>,
	/// The locals of the function being lowered that own their value.
	owned_locals: FxHashMap<SymbolId, Slot<'ctx>>,
	/// The generated drops and copies, by shape.
	drop_functions: std::cell::RefCell<FxHashMap<String, FunctionValue<'ctx>>>,
	copy_functions: std::cell::RefCell<FxHashMap<String, FunctionValue<'ctx>>>,
	/// The nodes reading each local (last uses).
	local_uses: std::cell::RefCell<FxHashMap<SymbolId, Vec<NodeId>>>,
	/// The parent of every node (last uses).
	node_parents: std::cell::RefCell<Vec<usize>>,
	/// Whether the program reports its leaks (`Options::leak_check`).
	leak_check: bool,
	/// `fa test`: the tests the program runs instead of its top level (one
	/// per run, chosen by `main`'s first argument: `testing.rs`).
	pub(crate) tests: Vec<(FileId, NodeId)>,
	/// The TBAA access tags of array headers and of values, and the type
	/// node of values ([`Lowering::tag`]).
	tbaa: std::cell::OnceCell<(
		inkwell::values::MetadataValue<'ctx>,
		inkwell::values::MetadataValue<'ctx>,
		inkwell::values::MetadataValue<'ctx>,
	)>,
	/// A block allocated for the box of a recursive field before its value
	/// was computed ([`Lowering::object_literal`]), which `box_rec` uses.
	pending_box: std::cell::Cell<Option<(PointerValue<'ctx>, Repr)>>,
	/// Whether a drop of the program does more than free memory (calls a
	/// `destroy`, releases a thread-safe value, disposes an observer): the
	/// drops at the end of `main` then run, else they are skipped.
	effectful_drops: std::cell::Cell<bool>,
	/// The drop flags of the locals holding a resource of foreign code (a
	/// type with `destroy`), by slot: cleared when the local moves out.
	resource_flags: std::cell::RefCell<FxHashMap<PointerValue<'ctx>, PointerValue<'ctx>>>,
	/// The `let` locals bound to `text.codePoints()` that are only indexed
	/// and measured, by symbol: the call, evaluated again (on an immutable
	/// text) where they are read, so that the array is never built.
	virtual_code_points: FxHashMap<SymbolId, NodeId>,
	/// The relation sides of the object types of the program (`relations.rs`).
	relation_sides: relations::RelationSides,
	/// The targets an object literal gave to relation sides, linked by the
	/// `add` that stores it.
	pending_links: Vec<(fa_analyzer::ids::Sym, Vec<PointerValue<'ctx>>)>,
	/// The locals a reference designates a part of (last uses).
	referenced: std::cell::RefCell<Option<rustc_hash::FxHashSet<SymbolId>>>,
	/// The split calls whose parts are views, per file (`views.rs`).
	split_views: std::cell::RefCell<FxHashMap<FileId, views::SplitViews>>,
	/// The reciprocal of each 64-bit `let` integer, computed where it is
	/// declared, by function (`division.rs`): pure, dropped when unused.
	reciprocals: FxHashMap<(SymbolId, FunctionValue<'ctx>), (IntValue<'ctx>, IntValue<'ctx>)>,
	/// The one stack slot per function that the slow path of an append
	/// spills a string header to (`text.rs`): its uses never overlap.
	append_headers: std::cell::RefCell<FxHashMap<FunctionValue<'ctx>, PointerValue<'ctx>>>,
	/// The slots holding the sources of the `reactive` values
	/// (`reactivity.rs`), the nodes of the derived values, and the sources
	/// given to the parameters of the function being lowered (D24).
	reactive_sources: FxHashMap<SymbolId, PointerValue<'ctx>>,
	derived_nodes: FxHashMap<SymbolId, PointerValue<'ctx>>,
	param_sources: FxHashMap<SymbolId, PointerValue<'ctx>>,
	/// Above 0 while code is evaluated again to designate a source: its
	/// reads are not tracked.
	untracked: u32,
	/// The alias scope domains made so far (`aliasing.rs`).
	alias_domains: u32,
	/// Async: the mode of the program, the async instances (`tasks.rs`).
	tasks: tasks::Tasks<'ctx>,
	/// Types whose construction and uses prove a single arena owner.
	tree_arenas: trees::TreeArenas,
	/// The payload words of each named union kept inline, `None` when boxed
	/// (`inline_unions.rs`).
	inline_unions: std::cell::RefCell<FxHashMap<fa_analyzer::ids::NominalId, Option<u32>>>,
	/// The spans of the bodies of the loops of the file, computed once
	/// (`division.rs`; per file, like the fields of `program::FileState`).
	loop_bodies: std::cell::OnceCell<Vec<fa_parser::SourceSpan>>,
	/// The unit being lowered, when the program is compiled per unit
	/// (`units.rs`); `None`: the whole program in one module.
	split: Option<Box<units::Split<'ctx>>>,
}

impl<'a, 'ctx> Lowering<'a, 'ctx> {
	/// A lowering of the files of a program (each with the path its stops
	/// show), the entry file last or alone, its module-level values
	/// initialized in `init_order`.
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn new(
		session: &'a Session,
		files: &[(FileId, String)],
		entry: FileId,
		init_order: Vec<(FileId, NodeId)>,
		context: &'ctx Context,
		module: &'a Module<'ctx>,
		target: Target,
		usize: IntType<'ctx>,
	) -> Self {
		let pointer = context.ptr_type(AddressSpace::default());
		let mut other_files = FxHashMap::default();
		for (file, path) in files {
			if *file != entry {
				other_files.insert(*file, program::FileState::new(session, *file, path));
			}
		}
		let path = files
			.iter()
			.find(|(file, _)| *file == entry)
			.map(|(_, path)| path.clone())
			.unwrap_or_default();
		let state = program::FileState::new(session, entry, &path);
		let mut lowering = Lowering {
			session,
			file: entry,
			module_file: false,
			other_files,
			init_order,
			entry,
			symbol_functions: FxHashMap::default(),
			exit_code: None,
			tree: state.tree,
			analysis: state.analysis,
			path: state.path.clone(),
			context,
			module,
			builder: context.create_builder(),
			runtime: Runtime::new(context, module, target, usize),
			target,
			usize,
			string_type: context.struct_type(&[pointer.into(), usize.into(), usize.into()], false),
			functions: FxHashMap::default(),
			slots: FxHashMap::default(),
			scope_owners: FxHashMap::default(),
			shared: FxHashMap::default(),
			nullable_references: FxHashMap::default(),
			reserved_pushes: FxHashMap::default(),
			reserved_appends: FxHashMap::default(),
			frame: None,
			constants: FxHashMap::default(),
			lines: state.lines.clone(),
			object_types: std::cell::RefCell::new(FxHashMap::default()),
			union_ids: std::cell::RefCell::new(FxHashMap::default()),
			type_ids: std::cell::RefCell::new(FxHashMap::default()),
			generic_functions: FxHashMap::default(),
			instances: FxHashMap::default(),
			pending_instances: Vec::new(),
			bindings: std::cell::RefCell::new(FxHashMap::default()),
			global_slots: FxHashMap::default(),
			invariant_globals: rustc_hash::FxHashSet::default(),
			overflow_checks: true,
			cleanups: std::cell::RefCell::new(Vec::new()),
			item_results: Vec::new(),
			pending_receiver: None,
			provided: FxHashMap::default(),
			references: FxHashMap::default(),
			saved_cleanups: Vec::new(),
			owned_locals: FxHashMap::default(),
			drop_functions: std::cell::RefCell::new(FxHashMap::default()),
			copy_functions: std::cell::RefCell::new(FxHashMap::default()),
			local_uses: std::cell::RefCell::new(FxHashMap::default()),
			node_parents: std::cell::RefCell::new(Vec::new()),
			leak_check: false,
			tests: Vec::new(),
			tbaa: std::cell::OnceCell::new(),
			pending_box: std::cell::Cell::new(None),
			effectful_drops: std::cell::Cell::new(false),
			resource_flags: std::cell::RefCell::new(FxHashMap::default()),
			virtual_code_points: FxHashMap::default(),
			relation_sides: relations::relation_sides(
				files.iter().map(|(file, _)| session.analysis(*file)),
			),
			pending_links: Vec::new(),
			referenced: std::cell::RefCell::new(None),
			split_views: std::cell::RefCell::new(FxHashMap::default()),
			reciprocals: FxHashMap::default(),
			append_headers: std::cell::RefCell::new(FxHashMap::default()),
			reactive_sources: FxHashMap::default(),
			derived_nodes: FxHashMap::default(),
			param_sources: FxHashMap::default(),
			untracked: 0,
			alias_domains: 0,
			tasks: tasks::Tasks::default(),
			tree_arenas: trees::TreeArenas::default(),
			inline_unions: std::cell::RefCell::new(FxHashMap::default()),
			loop_bodies: std::cell::OnceCell::new(),
			split: None,
		};
		lowering.module_file = state.module_file;
		lowering
	}

	/// Counts the live heap blocks and reports the leaks at the end
	/// (`Options::leak_check`).
	pub(crate) fn with_leak_check(mut self, enabled: bool) -> Self {
		self.leak_check = enabled;
		self.runtime = self.runtime.with_leak_check(enabled);
		self
	}

	/// `fa test`: lowers these tests instead of the top level (`testing.rs`).
	pub(crate) fn with_tests(mut self, tests: Vec<(FileId, NodeId)>) -> Self {
		self.tests = tests;
		self
	}

	/// Turns the integer overflow checks off (`--overflow-checks=false`).
	pub(crate) fn with_overflow_checks(mut self, enabled: bool) -> Self {
		self.overflow_checks = enabled;
		self
	}

	/// Whether integer overflows are checked.
	pub(crate) fn overflow_checks(&self) -> bool {
		self.overflow_checks
	}

	pub(crate) fn types(&self) -> &'a TypeStore {
		self.session.types()
	}

	/// Lowers the whole program: the runtime, the functions of every file,
	/// then `main` (the module-level values, the top level of the entry).
	pub(crate) fn lower(mut self) -> Result<(), Error> {
		self.runtime.link_tier1()?;
		// A materializing read must not race another thread holding the same
		// collection. Keep the eager representation in parallel/foreign and
		// multifile programs until that boundary normalizes captured arrays.
		let compact = self.other_files.is_empty()
			&& self.analysis.foreign_args.is_empty()
			&& !(0..self.tree.nodes.len()).any(|at| {
				matches!(
					self.view(NodeId::new(at)),
					View::For {
						parallel: Some(_),
						..
					}
				)
			});
		if let Some(flag) = self.module.get_global("fa_compact_characters") {
			flag.set_initializer(&self.context.i32_type().const_int(compact as u64, false));
			flag.set_constant(true);
			flag.set_linkage(Linkage::Internal);
		}
		let mut files: Vec<FileId> = self.other_files.keys().copied().collect();
		files.sort();
		files.push(self.entry);
		for &file in &files {
			self.switch_file(file);
			self.index_functions()?;
			self.find_shared_locals();
			let lambdas: Vec<NodeId> = self
				.functions
				.iter()
				.filter(|(_, function)| function.lambda)
				.map(|(node, _)| *node)
				.collect();
			for lambda in lambdas {
				let captures = self.find_captures(lambda);
				if let Some(function) = self.functions.get_mut(&lambda) {
					function.captures = captures;
				}
			}
			self.copy_cleanup_captures();
		}
		self.prepare_tree_arenas()?;
		for &file in &files {
			self.switch_file(file);
			let mut declarations: Vec<NodeId> = self.functions.keys().copied().collect();
			declarations.sort();
			for declaration in declarations {
				self.lower_function(declaration)?;
			}
		}
		self.switch_file(self.entry);
		if self.tests.is_empty() {
			self.lower_main()?;
		} else {
			self.lower_test_main()?;
		}
		loop {
			if let Some((file, instance)) = self.pending_instances.pop() {
				self.switch_file(file);
				self.lower_function(instance)?;
				continue;
			}
			// the async instances, requested by the code lowered so far
			if !self.lower_next_async_instance()? {
				break;
			}
		}
		self.switch_file(self.entry);
		self.define_coroutine_shims()?;

		self.define_format_union()?;
		self.define_boxes()?;
		// the drops at the end of `main` only free memory: the system takes
		// it back at exit (drops with an effect: `destroy`, thread-safe values)
		if let Some(flag) = self.module.get_global("fa.drops_at_exit") {
			let needed = self.effectful_drops.get();
			flag.set_initializer(&self.context.bool_type().const_int(needed as u64, false));
		}
		self.runtime.internalize()?;
		Ok(())
	}

	// ── locations and errors ────────────────────────────────────────────

	/// `file:line:column` of a node (1-based, the column in characters).
	pub(crate) fn location(&self, node: usize) -> String {
		let start = self.tree.spans[node].start;
		let line = self
			.lines
			.partition_point(|&line_start| line_start <= start);
		let line_start = self.lines[line - 1];
		let column = self.tree.source[line_start..start].chars().count() + 1;
		format!("{}:{line}:{column}", self.path)
	}

	pub(crate) fn unsupported(&self, node: usize, what: &str) -> Error {
		Error::Unsupported {
			message: what.to_string(),
			location: self.location(node),
		}
	}

	fn view(&self, node: NodeId) -> View<'a> {
		syntax::view(self.tree, node)
	}

	pub(crate) fn type_of(&self, node: NodeId) -> TypeId {
		self.analysis.type_of(node)
	}

	/// The check bits of a node; without overflow checks, never
	/// `OVERFLOW_NATIVE` nor `SINK_CHECK`.
	pub(crate) fn flags(&self, node: NodeId) -> NodeFlags {
		let flags = self.analysis.flags_of(node);
		if self.overflow_checks {
			flags
		} else {
			flags - NodeFlags::OVERFLOW_NATIVE - NodeFlags::SINK_CHECK
		}
	}

	/// The representation of a node's type.
	pub(crate) fn node_repr(&self, node: NodeId) -> Result<Repr, Error> {
		self.repr_of(self.type_of(node), node.index())
	}

	// ── constants ───────────────────────────────────────────────────────

	/// A private constant holding `bytes` (deduplicated).
	pub(crate) fn constant_bytes(&mut self, bytes: &[u8]) -> PointerValue<'ctx> {
		if let Some(pointer) = self.constants.get(bytes) {
			return *pointer;
		}
		let value = self.context.const_string(bytes, false);
		let global = self
			.module
			.add_global(value.get_type(), None, "fa.constant");
		global.set_initializer(&value);
		global.set_constant(true);
		global.set_linkage(Linkage::Private);
		global.set_unnamed_addr(true);
		global.set_alignment(1);
		let pointer = global.as_pointer_value();
		self.constants.insert(bytes.to_vec(), pointer);
		pointer
	}

	/// A string literal value.
	pub(crate) fn string_constant(&mut self, text: &str) -> Val<'ctx> {
		let pointer = self.constant_bytes(text.as_bytes());
		let length = self.usize.const_int(text.len() as u64, false);
		let value = self.string_type.const_named_struct(&[
			pointer.into(),
			length.into(),
			self.static_capacity(text).into(),
		]);
		Val::new(Repr::Str, value)
	}

	/// The `SIMPLE` flag of a string's capacity word: the top bit, set when
	/// every byte is ASCII other than '\r' (one byte per character).
	pub(crate) fn simple_flag(&self) -> IntValue<'ctx> {
		let bits = self.usize.get_bit_width();
		self.usize.const_int(1 << (bits - 1), false)
	}

	pub(crate) fn string_inline_flag(&self) -> IntValue<'ctx> {
		self.usize
			.const_int(1 << (self.usize.get_bit_width() - 2), false)
	}

	pub(crate) fn string_kind_mask(&self) -> IntValue<'ctx> {
		self.usize
			.const_int((1 << (self.usize.get_bit_width() - 2)) - 1, false)
	}

	/// The capacity word of a static string (kind 0, with its `SIMPLE` flag).
	pub(crate) fn static_capacity(&self, text: &str) -> IntValue<'ctx> {
		if text.bytes().all(|byte| byte < 0x80 && byte != b'\r') {
			self.simple_flag()
		} else {
			self.usize.const_zero()
		}
	}

	/// The site of a runtime check: two NUL-terminated strings, the subject
	/// (a type name, or the source of an assertion) and the location.
	pub(crate) fn site(&mut self, subject: &str, node: NodeId) -> PointerValue<'ctx> {
		let mut bytes = subject.as_bytes().to_vec();
		bytes.push(0);
		bytes.extend_from_slice(self.location(node.index()).as_bytes());
		bytes.push(0);
		self.constant_bytes(&bytes)
	}

	// ── blocks and the builder ──────────────────────────────────────────

	fn frame(&self) -> &Frame<'ctx> {
		self.frame.as_ref().expect("inside a function")
	}

	fn frame_mut(&mut self) -> &mut Frame<'ctx> {
		self.frame.as_mut().expect("inside a function")
	}

	pub(crate) fn append_block(&self, name: &str) -> BasicBlock<'ctx> {
		self.context.append_basic_block(self.frame().function, name)
	}

	/// After a terminator (`return`, `break`, a stop): continues in a block
	/// that nothing reaches, which the optimizer removes.
	pub(crate) fn continue_unreachable(&self) {
		let block = self.append_block("unreachable");
		self.builder.position_at_end(block);
	}

	/// Branches to `block` unless the current block already ended.
	fn branch_to(&self, block: BasicBlock<'ctx>) -> Result<(), Error> {
		let current = self.builder.get_insert_block().expect("a block");
		if current.get_terminator().is_none() {
			self.builder.build_unconditional_branch(block)?;
		}
		Ok(())
	}

	/// A new slot of an IR type in the entry block of the current function.
	fn alloca_type(
		&self,
		ty: BasicTypeEnum<'ctx>,
		name: &str,
	) -> Result<PointerValue<'ctx>, Error> {
		let builder = self.context.create_builder();
		let allocas = self.frame().allocas;
		match allocas.get_first_instruction() {
			Some(first) => builder.position_before(&first),
			None => builder.position_at_end(allocas),
		}
		Ok(builder.build_alloca(ty, name)?)
	}

	/// A new slot in the entry block of the current function.
	fn alloca(&self, repr: Repr, name: &str) -> Result<Slot<'ctx>, Error> {
		let builder = self.context.create_builder();
		let allocas = self.frame().allocas;
		match allocas.get_first_instruction() {
			Some(first) => builder.position_before(&first),
			None => builder.position_at_end(allocas),
		}
		let pointer = builder.build_alloca(self.llvm_type(repr), name)?;
		Ok(Slot { pointer, repr })
	}

	fn load(&self, slot: Slot<'ctx>) -> Result<Val<'ctx>, Error> {
		// an element of a columnar, gathered from its columns
		if let Repr::Row(element) = slot.repr {
			let value = self.gather(&self.builder, slot.pointer, element)?;
			let value =
				self.freeze_union_padding(&self.builder, Repr::Object(element), value.into());
			return Ok(Val::new(Repr::Object(element), value));
		}
		let value = self
			.builder
			.build_load(self.llvm_type(slot.repr), slot.pointer, "")?;
		if let Some(instruction) = inkwell::values::BasicValue::as_instruction_value(&value) {
			self.tag(instruction, false)?;
			// a module `let` read by a function: initialized before any call
			if self
				.frame
				.as_ref()
				.is_some_and(|frame| frame.owner.is_some())
				&& self.invariant_globals.contains(&slot.pointer)
			{
				let kind = self.context.get_kind_id("invariant.load");
				instruction
					.set_metadata(self.context.metadata_node(&[]), kind)
					.map_err(|error| Error::Llvm(error.to_string()))?;
			}
		}
		let value = self.freeze_union_padding(&self.builder, slot.repr, value);
		Ok(Val::new(slot.repr, value))
	}

	fn store(&self, slot: Slot<'ctx>, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		if let Repr::Row(element) = slot.repr {
			let value = self.convert(value, Repr::Object(element), node.index())?;
			if let Some(raw) = value.value {
				self.scatter(&self.builder, slot.pointer, element, raw)?;
			}
			return Ok(());
		}
		let value = self.convert(value, slot.repr, node.index())?;
		if let Some(raw) = value.value {
			let instruction = self.builder.build_store(slot.pointer, raw)?;
			self.tag(instruction, false)?;
		}
		Ok(())
	}

	/// Type-based alias analysis (TBAA): the header of an array (its items
	/// pointer, its length) is never a value of the program (a local, a
	/// field, an item), so a store into an item doesn't make LLVM reload the
	/// length or the items pointer, and bounds checks leave loops. A value
	/// of a scalar type (`fa i8`, `fa ptr`, `fa double`…, children of `fa
	/// value`) never shares its memory with a scalar of another type, since
	/// the language has no reinterpreting views: a store into an `Array(UInt8)`
	/// doesn't make LLVM reload the place holding another array (the
	/// destination of `blur`). An aggregate access is tagged `fa value`,
	/// which aliases every scalar. Accesses without a tag may alias anything
	/// (the runtime's, under clang's own root).
	pub(crate) fn tag(
		&self,
		instruction: inkwell::values::InstructionValue<'ctx>,
		header: bool,
	) -> Result<(), Error> {
		let (header_tag, value_tag, value_type) = *self.tbaa.get_or_init(|| {
			let root = self
				.context
				.metadata_node(&[self.context.metadata_string("fa").into()]);
			let zero = self.context.i64_type().const_zero();
			let access = |name: &str| {
				let node = self.context.metadata_node(&[
					self.context.metadata_string(name).into(),
					root.into(),
					zero.into(),
				]);
				(
					self.context
						.metadata_node(&[node.into(), node.into(), zero.into()]),
					node,
				)
			};
			let (header, _) = access("fa header");
			let (value, value_type) = access("fa value");
			(header, value, value_type)
		});
		let tag = if header {
			header_tag
		} else {
			let accessed = match instruction.get_opcode() {
				inkwell::values::InstructionOpcode::Load => {
					inkwell::types::BasicTypeEnum::try_from(instruction.get_type()).ok()
				}
				inkwell::values::InstructionOpcode::Store => instruction
					.get_operand(0)
					.and_then(|operand| operand.value())
					.map(|operand| operand.get_type()),
				_ => None,
			};
			let scalar = match accessed {
				Some(BasicTypeEnum::IntType(int)) => Some(format!("fa i{}", int.get_bit_width())),
				Some(BasicTypeEnum::FloatType(float)) => Some(
					if float == self.context.f32_type() {
						"fa float"
					} else {
						"fa double"
					}
					.to_string(),
				),
				Some(BasicTypeEnum::PointerType(_)) => Some("fa ptr".to_string()),
				_ => None,
			};
			match scalar {
				Some(name) => {
					let zero = self.context.i64_type().const_zero();
					let node = self.context.metadata_node(&[
						self.context.metadata_string(&name).into(),
						value_type.into(),
						zero.into(),
					]);
					self.context
						.metadata_node(&[node.into(), node.into(), zero.into()])
				}
				None => value_tag,
			}
		};
		instruction
			.set_metadata(tag, self.context.get_kind_id("tbaa"))
			.map_err(|error| Error::Llvm(error.to_string()))
	}

	/// A call returning a value.
	pub(crate) fn call_value(
		&self,
		function: FunctionValue<'ctx>,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<inkwell::values::BasicValueEnum<'ctx>, Error> {
		Ok(self
			.builder
			.build_call(function, arguments, "")?
			.try_as_basic_value()
			.basic()
			.expect("a function with a result"))
	}

	/// A call of a tier 1 runtime function returning nothing.
	pub(crate) fn call_runtime(
		&self,
		name: &str,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<(), Error> {
		let function = self.runtime.function(name)?;
		self.builder.build_call(function, arguments, "")?;
		Ok(())
	}

	/// A call of a `noreturn` runtime function, then an unreachable continuation.
	pub(crate) fn call_noreturn(
		&self,
		name: &str,
		arguments: &[BasicMetadataValueEnum<'ctx>],
	) -> Result<(), Error> {
		self.call_runtime(name, arguments)?;
		self.builder.build_unreachable()?;
		Ok(())
	}

	/// Branches to a cold block when `failed`, built by `cold` (which ends
	/// with a `noreturn` call), and continues in the other block.
	pub(crate) fn guard(
		&mut self,
		failed: IntValue<'ctx>,
		cold: impl FnOnce(&mut Self) -> Result<(), Error>,
	) -> Result<(), Error> {
		let fail = self.append_block("check_failed");
		let pass = self.append_block("check_passed");
		let branch = self.builder.build_conditional_branch(failed, fail, pass)?;
		self.weigh(branch, 1, 1 << 20)?;
		self.builder.position_at_end(fail);
		cold(self)?;
		self.builder.position_at_end(pass);
		Ok(())
	}

	/// A conditional branch whose first target is almost always taken.
	pub(crate) fn likely(
		&self,
		branch: inkwell::values::InstructionValue<'ctx>,
	) -> Result<(), Error> {
		self.weigh(branch, 1 << 20, 1)
	}

	/// The `!prof` branch weights of a conditional branch.
	fn weigh(
		&self,
		branch: inkwell::values::InstructionValue<'ctx>,
		taken: u64,
		other: u64,
	) -> Result<(), Error> {
		let i32 = self.context.i32_type();
		let weights = self.context.metadata_node(&[
			self.context.metadata_string("branch_weights").into(),
			i32.const_int(taken, false).into(),
			i32.const_int(other, false).into(),
		]);
		branch
			.set_metadata(weights, self.context.get_kind_id("prof"))
			.map_err(|error| Error::Llvm(error.to_string()))
	}

	// ── functions ───────────────────────────────────────────────────────

	/// The cleanup of a `watch` block sees the reactive values of its run:
	/// it copies them, as it copies the rest.
	fn copy_cleanup_captures(&mut self) {
		let cleanups: Vec<NodeId> = self
			.analysis
			.watches
			.values()
			.filter_map(|watch| watch.cleanup)
			.collect();
		for cleanup in cleanups {
			let mut reactive = Vec::new();
			let mut stack = vec![cleanup.index()];
			while let Some(index) = stack.pop() {
				if let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(index))
					&& self.analysis.symbols.symbol(symbol).kind == SymbolKind::Reactive
				{
					reactive.push(symbol);
				}
				self.tree.nodes[index].for_each_child(|child| stack.push(child));
			}
			if let Some(function) = self.functions.get_mut(&cleanup) {
				for symbol in reactive {
					if !function.captures.contains(&symbol) {
						function.captures.push(symbol);
					}
				}
				function.by_reference = false;
			}
		}
	}

	/// Declares every function declaration of the file.
	fn index_functions(&mut self) -> Result<(), Error> {
		let mut infos: Vec<(NodeId, FnInfo)> = self
			.analysis
			.functions
			.iter()
			.filter(|(node, info)| {
				info.declaration == Some(**node)
					|| (info.declaration.is_none() && info.lambda == **node)
			})
			.map(|(node, info)| (*node, info.clone()))
			.collect();
		infos.sort_by_key(|(node, _)| *node);
		for (node, _) in &infos {
			let info = &self.analysis.functions[node];
			self.scope_owners.insert(info.scope, Some(*node));
		}
		for (node, info) in infos {
			if let Some(symbol) = info.symbol {
				self.symbol_functions.insert(symbol, node);
			}
			if !info.generics.is_empty() {
				// declared per instance, when called (`instance`)
				self.generic_functions.insert(node, info);
				continue;
			}
			let function = self.declare_function(node, info, "")?;
			self.functions.insert(node, function);
		}
		Ok(())
	}

	/// Whether a parameter of this representation is passed as a pointer to
	/// the caller's value (never copied): every aggregate but a function
	/// value, outside of asynchronous programs (whose tasks may outlive the
	/// caller's places). The same for direct and indirect calls.
	pub(crate) fn borrows_parameter(&self, repr: Repr) -> bool {
		!self.tasks.asynchronous
			&& !matches!(repr, Repr::Function(_))
			&& self.llvm_type(repr).is_struct_type()
	}

	/// Declares a function (or an instance of a generic one, under the
	/// current bindings): its IR signature.
	pub(crate) fn declare_function(
		&mut self,
		node: NodeId,
		info: FnInfo,
		suffix: &str,
	) -> Result<Function<'ctx>, Error> {
		{
			let is_lambda = info.declaration.is_none();
			let captures = self.find_captures(info.lambda);
			let lambda = is_lambda || !captures.is_empty();
			// a lambda sees the variables themselves only when it is given
			// directly to a call (a callback) and doesn't escape from it
			let given_to_call = info.declaration.is_some() || {
				let parent = self.parents().get(info.lambda.index()).copied();
				parent.is_some_and(|parent| {
					parent != usize::MAX
						&& matches!(self.view(NodeId::new(parent)), View::Call { .. })
				})
			};
			let by_reference = !self.analysis.escapes.contains(&info.lambda) && given_to_call;
			let Some(sig) = info.sig else {
				return Err(self.unsupported(node.index(), "a function without a signature"));
			};
			let sig = self.types().sig(sig);
			let ret = self.repr_of(sig.ret, node.index())?;
			let mut params = Vec::new();
			let mut types: Vec<BasicMetadataTypeEnum> = Vec::new();
			if lambda {
				types.push(self.pointer_type().into());
			}
			// a parameter the function changes is a reference to the
			// caller's place (memory management page: parameters are references)
			let mut by_pointer = vec![false; info.params.len()];
			let mut borrowed = vec![false; info.params.len()];
			if !is_lambda {
				let (mutates, invalidates) = self.types().effects(sig.effects);
				for path in mutates.iter().chain(invalidates) {
					let (data, _) = self.types().path_data(*path);
					let index = match data.root {
						fa_analyzer::types::PathRoot::Param(index) => index as usize,
						fa_analyzer::types::PathRoot::SelfParam => 0,
						_ => continue,
					};
					if let Some(flag) = by_pointer.get_mut(index) {
						*flag = true;
					}
				}
			}
			for (position, param) in info.params.iter().enumerate() {
				let ty = self.analysis.symbols.symbol(param.symbol).ty;
				let repr = self.repr_of(ty, param.node.index())?;
				if !repr.has_value() {
					return Err(self.unsupported(param.node.index(), "this parameter type"));
				}
				// Parameters are never copied: synchronous calls, direct or through
				// a function value, borrow aggregates instead of expanding them
				// into registers and spilling a new copy in every callee.
				if !by_pointer[position] && self.borrows_parameter(repr) {
					by_pointer[position] = true;
					borrowed[position] = true;
				}
				if by_pointer[position] {
					if param.default.is_some() && !borrowed[position] {
						return Err(self.unsupported(
							param.node.index(),
							"a changed parameter with a default value",
						));
					}
					// a borrowed parameter with a default: the caller's place,
					// or null and a false flag for the default (a lambda's
					// callers give every argument)
					let has_default = param.default.is_some() && !is_lambda;
					types.push(self.pointer_type().into());
					if has_default {
						types.push(self.context.bool_type().into());
					}
					params.push((repr, has_default));
					continue;
				}
				types.push(self.llvm_type(repr).into());
				let has_default = param.default.is_some() && !is_lambda;
				if has_default {
					types.push(self.context.bool_type().into());
				}
				params.push((repr, has_default));
			}
			// the requirements: one hidden pointer each, after the parameters
			for _ in self.types().requirements(sig.requires) {
				types.push(self.pointer_type().into());
			}
			let throws = info.throws != TypeId::NEVER;
			let fn_type = match (ret.has_value(), throws) {
				(true, false) => self.llvm_type(ret).fn_type(&types, false),
				(false, false) => self.context.void_type().fn_type(&types, false),
				(true, true) => self
					.context
					.struct_type(&[self.llvm_type(ret), self.pointer_type().into()], false)
					.fn_type(&types, false),
				(false, true) => self.pointer_type().fn_type(&types, false),
			};
			let name = info
				.symbol
				.map(|symbol| {
					let symbol = self.analysis.symbols.symbol(symbol);
					self.session.types().syms.resolve(symbol.name).to_string()
				})
				.map(|name| {
					if is_lambda {
						"lambda".to_string()
					} else {
						name
					}
				})
				.unwrap_or_else(|| "lambda".to_string());
			let value = self.module.add_function(
				&format!("fa.{name}{suffix}"),
				fn_type,
				Some(Linkage::Internal),
			);
			value.set_call_conventions(FAST_CALLING_CONVENTION);
			self.runtime.add_attributes(value, &["nounwind"]);
			if self.is_split() && suffix.is_empty() && !is_lambda {
				// a function other modules call: its symbol (`units.rs`)
				self.export_function(node, value);
			}
			// Tiny leaf helpers often contain a generic runtime path (for
			// example `items.get(i) ?? fallback`). Inline them where the
			// caller's types and constants can eliminate that path.
			// By size (syntax nodes): a leaf (no call but of built-ins) of up to
			// 64 nodes and at most one loop, like `unlink` of a linked list
			// stored in an array or a loop escaping a string, is always inlined;
			// a function of up to 96 nodes gets the hint. Nested loops pay for
			// their call (and a kernel stays one function).
			let mut leaf = vec![info.body.index()];
			let mut calls = false;
			let mut loops = 0;
			let mut count = 0;
			while let Some(at) = leaf.pop() {
				count += 1;
				if count > 96 {
					break;
				}
				if matches!(self.view(NodeId::new(at)), View::Call { .. })
					&& !self.analysis.call(NodeId::new(at)).is_some_and(|call| {
						matches!(
							call.callee,
							fa_analyzer::analysis::Callee::Builtin(_)
								| fa_analyzer::analysis::Callee::Conversion(_)
						)
					}) {
					calls = true;
				}
				if matches!(
					self.view(NodeId::new(at)),
					View::For { .. } | View::While { .. } | View::WhileLet { .. } | View::Loop(_)
				) {
					loops += 1;
				}
				self.tree.nodes[at].for_each_child(|child| leaf.push(child));
			}
			if !self.tasks.asynchronous {
				if !calls && (count <= 24 || (count <= 64 && loops <= 1)) {
					self.runtime.add_attributes(value, &["alwaysinline"]);
				} else if count <= 96 {
					self.runtime.add_attributes(value, &["inlinehint"]);
				}
			}
			for index in 0..types.len() {
				value.add_attribute(
					AttributeLoc::Param(index as u32),
					self.runtime.attribute("noundef"),
				);
			}
			if ret == Repr::Never && !throws {
				self.runtime.add_attributes(value, &["noreturn"]);
			}
			Ok(Function {
				value,
				arena_body: None,
				info,
				params,
				by_pointer,
				borrowed,
				ret,
				throws,
				lambda,
				by_reference,
				captures,
				declaration: node,
				bindings: self.bindings.borrow().clone(),
			})
		}
	}

	/// The function declaration owning a scope (`None`: the top level).
	fn owner_of_scope(&mut self, scope: ScopeId) -> Option<NodeId> {
		if let Some(owner) = self.scope_owners.get(&scope) {
			return *owner;
		}
		let data = self.analysis.symbols.scope(scope);
		let owner = match data.parent {
			Some(parent) if data.kind != ScopeKind::Module => self.owner_of_scope(parent),
			_ => None,
		};
		self.scope_owners.insert(scope, owner);
		owner
	}

	/// Finds the top-level locals that functions read or write: they become globals.
	fn find_shared_locals(&mut self) {
		let mut bodies: Vec<(NodeId, NodeId)> = self
			.functions
			.iter()
			.map(|(node, function)| (*node, function.info.lambda))
			.chain(
				self.generic_functions
					.iter()
					.map(|(node, info)| (*node, info.lambda)),
			)
			.collect();
		bodies.sort();
		for (owner, lambda) in bodies {
			let mut stack = vec![lambda.index()];
			while let Some(node) = stack.pop() {
				if let Resolution::Local(symbol) = self.analysis.resolution(NodeId::new(node)) {
					let data = self.analysis.symbols.symbol(symbol);
					let variable = !matches!(
						data.kind,
						SymbolKind::Function | SymbolKind::Type | SymbolKind::Namespace
					);
					if variable {
						let symbol_owner = self.owner_of_scope(data.scope);
						if symbol_owner.is_none() && symbol_owner != Some(owner) {
							self.shared.insert(symbol, ());
						}
					}
				}
				self.tree.nodes[node].for_each_child(|child| stack.push(child));
			}
		}
		self.mark_module_values();
	}

	/// Starts lowering a function: its allocas block and its first block.
	fn begin_function(&mut self, function: FunctionValue<'ctx>, ret: Repr, owner: Option<NodeId>) {
		let allocas = self.context.append_basic_block(function, "allocas");
		let start = self.context.append_basic_block(function, "start");
		self.builder.position_at_end(start);
		let outer = self.cleanups.take();
		self.saved_cleanups.push(outer);
		self.frame = Some(Frame {
			function,
			arena: false,
			same_value_zero: false,
			allocas,
			ret,
			owner,
			loops: Vec::new(),
			catches: Vec::new(),
			throws: false,
			requirements: Vec::new(),
			task: tasks::FrameTask::default(),
		});
	}

	/// Ends the function: the allocas block falls into the first block.
	fn end_function(&mut self) -> Result<(), Error> {
		let frame = self.frame.take().expect("inside a function");
		let outer = self.saved_cleanups.pop().unwrap_or_default();
		self.cleanups.replace(outer);
		let builder = self.context.create_builder();
		builder.position_at_end(frame.allocas);
		let start = frame
			.allocas
			.get_next_basic_block()
			.expect("the first block");
		builder.build_unconditional_branch(start)?;
		// the blocks that nothing reaches (after a `return`, a `break`, a stop)
		let mut block = frame.function.get_first_basic_block();
		while let Some(current) = block {
			if current.get_terminator().is_none() {
				builder.position_at_end(current);
				builder.build_unreachable()?;
			}
			block = current.get_next_basic_block();
		}
		Ok(())
	}

	fn lower_function(&mut self, key: NodeId) -> Result<(), Error> {
		// the locals of a function are its own; an instance has its bindings
		let saved_slots = std::mem::take(&mut self.slots);
		let saved_owned = std::mem::take(&mut self.owned_locals);
		let saved_bindings = self.bindings.replace(self.functions[&key].bindings.clone());
		let result = self.lower_function_body(key);
		self.slots = saved_slots;
		self.owned_locals = saved_owned;
		self.bindings.replace(saved_bindings);
		result
	}

	fn lower_function_body(&mut self, key: NodeId) -> Result<(), Error> {
		// the function, or its async instance (`tasks.rs`)
		let function = self.function_to_lower(key);
		let declaration = function.declaration;
		let (value, info, params, ret) = (
			function.arena_body.unwrap_or(function.value),
			function.info.clone(),
			function.params.clone(),
			function.ret,
		);
		let throws = function.throws;
		let lambda = function.lambda;
		let by_reference = function.by_reference;
		let captures = function.captures.clone();
		self.begin_function(value, ret, Some(declaration));
		self.frame_mut().arena = function.arena_body.is_some();
		self.frame_mut().throws = throws;
		self.begin_function_task(&function)?;
		let mut saved = Vec::new();
		if lambda && !captures.is_empty() {
			let environment = value
				.get_nth_param(0)
				.expect("an environment")
				.into_pointer_value();
			saved = self.bind_captures(environment, &captures, by_reference)?;
		}
		// the parameters, then their defaults in order
		let mut index = u32::from(lambda);
		let by_pointer = function.by_pointer.clone();
		for (position, (param, (repr, has_default))) in info.params.iter().zip(params).enumerate() {
			if by_pointer[position] {
				// the caller's place itself
				let pointer = value
					.get_nth_param(index)
					.expect("a parameter")
					.into_pointer_value();
				index += 1;
				let pointer = match (has_default, param.default) {
					(true, Some(default)) => {
						// not given: the default, in a slot the function owns
						let flag = value.get_nth_param(index).expect("a flag").into_int_value();
						index += 1;
						let own = self.alloca(repr, "default_parameter")?;
						let owned = self
							.needs_drop(repr)
							.then(|| self.alloca(Repr::Bool, "default"))
							.transpose()?;
						if let Some(owned) = owned {
							self.builder.build_store(
								owned.pointer,
								self.context.bool_type().const_zero(),
							)?;
						}
						let given_block = self.builder.get_insert_block().expect("a block");
						let use_default = self.append_block("default");
						let join = self.append_block("given");
						self.builder
							.build_conditional_branch(flag, join, use_default)?;
						self.builder.position_at_end(use_default);
						let default_value =
							self.region(Some(repr), default, |this| this.expr(default))?;
						self.store(own, default_value, default)?;
						if let Some(owned) = owned {
							self.builder.build_store(
								owned.pointer,
								self.context.bool_type().const_int(1, false),
							)?;
						}
						let default_end = self.builder.get_insert_block().expect("a block");
						self.branch_to(join)?;
						self.builder.position_at_end(join);
						let chosen = self.builder.build_phi(self.pointer_type(), "parameter")?;
						chosen
							.add_incoming(&[(&pointer, given_block), (&own.pointer, default_end)]);
						if let Some(owned) = owned {
							self.own_flagged(own, owned.pointer);
						}
						chosen.as_basic_value().into_pointer_value()
					}
					_ => pointer,
				};
				self.slots.insert(param.symbol, Slot { pointer, repr });
				continue;
			}
			let slot = self.alloca(repr, "parameter")?;
			self.slots.insert(param.symbol, slot);
			let given = value.get_nth_param(index).expect("a parameter");
			index += 1;
			match (has_default, param.default) {
				(true, Some(default)) => {
					let flag = value.get_nth_param(index).expect("a flag").into_int_value();
					index += 1;
					let use_default = self.append_block("default");
					let join = self.append_block("given");
					self.builder.build_store(slot.pointer, given)?;
					// the parameter owns its value when it is the default
					let owned = self
						.needs_drop(repr)
						.then(|| self.alloca(Repr::Bool, "default"));
					let owned = owned.transpose()?;
					if let Some(owned) = owned {
						self.builder
							.build_store(owned.pointer, self.context.bool_type().const_zero())?;
					}
					self.builder
						.build_conditional_branch(flag, join, use_default)?;
					self.builder.position_at_end(use_default);
					let default_value =
						self.region(Some(repr), default, |this| this.expr(default))?;
					self.store(slot, default_value, default)?;
					if let Some(owned) = owned {
						self.builder.build_store(
							owned.pointer,
							self.context.bool_type().const_int(1, false),
						)?;
					}
					self.branch_to(join)?;
					self.builder.position_at_end(join);
					if let Some(owned) = owned {
						self.own_flagged(slot, owned.pointer);
					}
				}
				_ => {
					self.builder.build_store(slot.pointer, given)?;
				}
			}
		}
		if let Some(sig) = info.sig {
			let requires = self.types().sig(sig).requires;
			let keys: Vec<TypeId> = self
				.types()
				.requirements(requires)
				.iter()
				.map(|requirement| requirement.ty)
				.collect();
			for key in keys {
				let pointer = value
					.get_nth_param(index)
					.expect("a requirement")
					.into_pointer_value();
				index += 1;
				self.frame_mut().requirements.push((pointer, key));
			}
		}
		// the sources of the reactive places its caller gave (D24)
		let params: Vec<SymbolId> = info.params.iter().map(|param| param.symbol).collect();
		self.receive_argument_sources(declaration, &params)?;
		// a started task waits here until its first turn
		self.initial_suspend(&info.params, &by_pointer)?;
		let result = if info.block_body {
			self.block(info.body, ret.has_value())?
		} else {
			self.expr(info.body)?
		};
		self.build_return(result, info.body)?;
		self.cleanups.borrow_mut().clear();
		for (symbol, slot) in saved {
			match slot {
				Some(slot) => self.slots.insert(symbol, slot),
				None => self.slots.remove(&symbol),
			};
		}
		self.end_function_task()?;
		self.end_function()
	}

	/// The drops at the end of `main`, behind `fa.drops_at_exit`, a constant
	/// decided once every drop function is known ([`Lowering::lower`]): when
	/// they only free memory, LLVM removes them (the system takes the memory
	/// back; leak-checked builds always drop).
	fn drop_at_exit(&mut self) -> Result<(), Error> {
		let flag = match self.module.get_global("fa.drops_at_exit") {
			Some(flag) => flag,
			None => {
				let flag =
					self.module
						.add_global(self.context.bool_type(), None, "fa.drops_at_exit");
				flag.set_initializer(&self.context.bool_type().const_int(1, false));
				flag.set_constant(true);
				flag.set_linkage(Linkage::Private);
				flag
			}
		};
		let needed = self
			.builder
			.build_load(self.context.bool_type(), flag.as_pointer_value(), "")?
			.into_int_value();
		let drops = self.append_block("drops_at_exit");
		let after = self.append_block("after_drops");
		self.builder
			.build_conditional_branch(needed, drops, after)?;
		self.builder.position_at_end(drops);
		self.drop_to(0)?;
		if !self.terminated() {
			self.builder.build_unconditional_branch(after)?;
		}
		self.builder.position_at_end(after);
		Ok(())
	}

	/// Returns from the current function with a value converted to its result.
	fn build_return(&mut self, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		if value.repr == Repr::Never {
			self.builder.build_unreachable()?;
			return Ok(());
		}
		let ret = self.frame().ret;
		// the result is owned by the caller; what the function owns is dropped
		let value = if ret.has_value() && value.repr.has_value() {
			self.owned_as(value, ret, Some(node))?
		} else {
			value
		};
		let top_level = self.frame().owner.is_none() && !self.in_task_body();
		if top_level && !self.leak_check && !self.in_coroutine() {
			self.drop_at_exit()?;
		} else {
			self.drop_to(0)?;
		}
		if top_level && self.leak_check && !self.in_coroutine() {
			self.call_runtime("fa_leak_check", &[])?;
		}
		if self.frame().throws {
			let null = self.pointer_type().const_null();
			if ret.has_value() {
				let value = self.convert(value, ret, node.index())?;
				let Some(raw) = value.value else {
					self.builder.build_unreachable()?;
					return Ok(());
				};
				let ty = self.raw_return_type().expect("a pair");
				let pair = self.builder.build_insert_value(
					ty.into_struct_type().get_undef(),
					raw,
					0,
					"",
				)?;
				let pair = self.builder.build_insert_value(pair, null, 1, "")?;
				self.return_raw(Some(inkwell::values::BasicValue::as_basic_value_enum(
					&pair,
				)))?;
			} else {
				self.return_raw(Some(null.into()))?;
			}
			return Ok(());
		}
		if top_level && self.in_coroutine() {
			// the top level of an asynchronous program: `main` exits after
			// the event loop
			self.store_exit_status()?;
			return self.return_raw(None);
		}
		if top_level {
			// the top level: flush and exit normally (with the entry
			// function's exit code)
			self.call_runtime("fa_flush", &[])?;
			if let Some(code) = self.exit_code {
				let code = self.builder.build_load(self.context.i32_type(), code, "")?;
				let exit = self.runtime.exit()?;
				self.builder.build_call(exit, &[code.into()], "")?;
			}

			match self.target {
				Target::Wasm => self.builder.build_return(None)?,
				_ => self
					.builder
					.build_return(Some(&self.context.i32_type().const_zero()))?,
			};
			return Ok(());
		}
		if ret.has_value() {
			let value = self.convert(value, ret, node.index())?;
			match value.value {
				Some(raw) => self.return_raw(Some(raw))?,
				None => {
					self.builder.build_unreachable()?;
				}
			};
		} else {
			self.return_raw(None)?;
		}
		Ok(())
	}

	/// `main(argc, argv)`, or `_start()` on wasm: the top level of the file
	/// (in a coroutine run by the event loop, in an asynchronous program).
	fn lower_main(&mut self) -> Result<(), Error> {
		if self.tasks.asynchronous {
			return self.lower_async_main();
		}
		let function = self.declare_main();
		self.begin_function(function, Repr::Void, None);
		self.lower_top_level(function)?;
		self.end_function()
	}

	/// Declares `main(argc, argv)`, or `_start()` on wasm.
	fn declare_main(&self) -> FunctionValue<'ctx> {
		let i32 = self.context.i32_type();
		let pointer = self.context.ptr_type(AddressSpace::default());
		let function = match self.target {
			Target::Wasm => self.module.add_function(
				"_start",
				self.context.void_type().fn_type(&[], false),
				Some(Linkage::External),
			),
			_ => self.module.add_function(
				"main",
				i32.fn_type(&[i32.into(), pointer.into()], false),
				Some(Linkage::External),
			),
		};
		self.runtime.add_attributes(function, &["nounwind"]);
		function
	}

	/// The top level of the entry file, in the current function (`main`,
	/// whose arguments `function` has, or the coroutine of the top level).
	fn lower_top_level(&mut self, function: FunctionValue<'ctx>) -> Result<(), Error> {
		if !self.in_coroutine() {
			self.start_program(function)?;
		}
		// the module-level values of the modules, in the program's order
		self.initialize_modules()?;
		let root = NodeId::new(self.tree.root);
		let View::Module { statements } = self.view(root) else {
			return Err(self.unsupported(self.tree.root, "this file"));
		};
		let statements: Vec<NodeId> = statements
			.iter()
			.filter(|&statement| {
				// the module-level values are initialized above
				!(self.module_file
					&& matches!(
						self.view(statement),
						View::Declaration { .. } | View::ExportValue { .. }
					))
			})
			.collect();
		self.top_level_statements(&statements)?;
		let mark = self.mark();
		self.call_entry_function()?;
		self.end_statement(mark, None)?;
		self.release_reactivity()?;
		self.build_return(Val::void(), root)
	}

	/// The start of `main`: the stack guard, the arguments.
	fn start_program(&mut self, function: FunctionValue<'ctx>) -> Result<(), Error> {
		let i32 = self.context.i32_type();
		let pointer = self.context.ptr_type(AddressSpace::default());
		let has_functions = !self.functions.is_empty()
			|| self.other_files.values().any(|state| state.has_functions());
		if self.target == Target::Native && has_functions {
			// a stack overflow is a stop, as on JavaScript
			self.call_runtime("fa_guard_stack", &[])?;
		}
		if self.uses_arguments() {
			let (argc, argv) = match self.target {
				Target::Wasm => (i32.const_zero().into(), pointer.const_null().into()),
				_ => (
					function.get_nth_param(0).expect("argc").into(),
					function.get_nth_param(1).expect("argv").into(),
				),
			};
			self.call_runtime("fa_arguments_init", &[argc, argv])?;
		}
		Ok(())
	}

	/// Whether the program reads `arguments`, or has an entry function.
	fn uses_arguments(&self) -> bool {
		let reads = |analysis: &Analysis| {
			analysis.resolutions.iter().any(|resolution| {
				matches!(resolution, Resolution::Builtin(id)
					if fa_analyzer::builtins::get(*id).path == "arguments")
			})
		};
		reads(self.analysis)
			|| self.other_files.values().any(|state| reads(state.analysis))
			|| self
				.session
				.source_file(self.file)
				.interface
				.export
				.is_some_and(|export| export.kind == SymbolKind::Function)
	}

	// ── locals ──────────────────────────────────────────────────────────

	/// The slot of a local, created on first use: an alloca of the function
	/// owning it, or a global for a top-level local that functions read.
	fn slot_of(&mut self, symbol: SymbolId, node: NodeId) -> Result<Slot<'ctx>, Error> {
		if let Some(&(holder, repr)) = self.references.get(&symbol) {
			let pointer = self
				.builder
				.build_load(self.pointer_type(), holder, "")?
				.into_pointer_value();
			return Ok(Slot { pointer, repr });
		}
		if let SymbolKind::Requirement { index } = self.analysis.symbols.symbol(symbol).kind {
			let Some(&(pointer, key)) = self.frame().requirements.get(index as usize) else {
				return Err(self.unsupported(node.index(), "this requirement"));
			};
			let repr = self.repr_of(key, node.index())?;
			return Ok(Slot { pointer, repr });
		}
		if let Some(slot) = self.slots.get(&symbol) {
			return Ok(*slot);
		}
		if let Some(slot) = self.global_slots.get(&symbol) {
			return Ok(*slot);
		}
		let data = self.analysis.symbols.symbol(symbol);
		let repr = self.repr_of(data.ty, node.index())?;
		if !repr.has_value() {
			return Err(self.unsupported(node.index(), "a local without a value"));
		}
		let owner = self.owner_of_scope(data.scope);
		if matches!(data.kind, SymbolKind::Use | SymbolKind::MutableUse)
			&& self.shared.contains_key(&symbol)
		{
			// a top-level reference that functions read: its address in a global
			self.reference_holder(symbol, repr);
			return self.slot_of(symbol, node);
		}
		let slot = if self.shared.contains_key(&symbol) {
			let ty = self.llvm_type(repr);
			let constant = self.immutable_scalar(symbol)?;
			let global = self.module.add_global(ty, None, "fa.global");
			global.set_linkage(Linkage::Internal);
			self.export_global(symbol, global);
			if let Some(value) = constant {
				global.set_initializer(&value.value.expect("a scalar constant"));
				global.set_constant(true);
			} else {
				global.set_initializer(&zero_of(ty));
				if data.kind == SymbolKind::Let && !self.reactive_sources.contains_key(&symbol) {
					self.invariant_globals.insert(global.as_pointer_value());
				}
			}
			let slot = Slot {
				pointer: global.as_pointer_value(),
				repr,
			};
			self.global_slots.insert(symbol, slot);
			return Ok(slot);
		} else if owner == self.frame().owner {
			self.alloca(repr, "local")?
		} else {
			return Err(self.unsupported(
				node.index(),
				"a function reading a local of the function around it (a closure)",
			));
		};
		self.slots.insert(symbol, slot);
		Ok(slot)
	}

	/// An immutable binding with a scalar initializer that needs no runtime
	/// evaluation. Its storage can live in read-only data from the start,
	/// rather than being initialized by `main` and reloaded after calls.
	fn immutable_scalar(&mut self, symbol: SymbolId) -> Result<Option<Val<'ctx>>, Error> {
		use fa_interpreter::Value;
		let data = self.analysis.symbols.symbol(symbol);
		if data.kind != SymbolKind::Let {
			return Ok(None);
		}
		let ty = data.ty;
		let View::Declaration {
			value: Some(node), ..
		} = self.view(data.decl_node)
		else {
			return Ok(None);
		};
		// the analyzer's constant expressions (literals, constant arithmetic,
		// templates, other constants), then the values computed at compile time
		let module_constant = self.analysis.module_constant(symbol).cloned();
		let constant = module_constant
			.or_else(|| self.comptime_value(node))
			.or_else(|| match self.view(node) {
				View::Integer(value) => Some(Value::Integer(value)),
				View::Float(value) => Some(Value::Float(value)),
				View::Boolean(value) => Some(Value::Boolean(value)),
				View::String(value) => Some(Value::String(value.into())),
				_ => None,
			});
		let Some(
			value @ (Value::Integer(_) | Value::Float(_) | Value::Boolean(_) | Value::String(_)),
		) = constant
		else {
			return Ok(None);
		};
		// Optional/composite conversions may need instructions. Only plain
		// scalar constants are valid LLVM global initializers here.
		if !matches!(
			self.repr_of(ty, node.index())?,
			Repr::Int(_) | Repr::F64 | Repr::F32 | Repr::Bool | Repr::Str
		) {
			return Ok(None);
		}
		self.constant_value(&value, ty, node).map(Some)
	}

	/// A compile-time value (`Analysis::constant`): scalars and strings as
	/// constants, the other values built ([`Lowering::constant_value`]).
	fn compile_time_scalar(&mut self, node: NodeId) -> Result<Option<Val<'ctx>>, Error> {
		use fa_interpreter::Value;
		// a `static` declaration holds its value: it lowers to nothing
		if let View::Declaration {
			kind: fa_analyzer::syntax::BindingKind::Static,
			..
		} = self.view(node)
		{
			return Ok(None);
		}
		let Some(value) = self.comptime_value(node) else {
			return Ok(None);
		};
		Ok(Some(match value {
			Value::Integer(integer) => self.integer_literal(integer, node)?,
			Value::Boolean(value) => Val::new(
				Repr::Bool,
				self.context.bool_type().const_int(value as u64, false),
			),
			Value::Float(value) => match self.node_repr(node)? {
				Repr::F32 => Val::new(
					Repr::F32,
					self.context.f32_type().const_float(value as f32 as f64),
				),
				_ => Val::new(Repr::F64, self.context.f64_type().const_float(value)),
			},
			Value::String(text) => self.string_constant(&text),
			Value::None => Val {
				repr: Repr::None,
				value: None,
			},
			value => {
				let ty = self.type_of(node);
				let built = self.constant_value(&value, ty, node)?;
				if self.needs_drop(built.repr) {
					self.fresh(built)
				} else {
					built
				}
			}
		}))
	}

	// ── statements ──────────────────────────────────────────────────────

	/// Lowers a statement, dropping its value.
	fn statement(&mut self, node: NodeId) -> Result<(), Error> {
		self.expr(node)?;
		Ok(())
	}

	/// The statements of a block; its value is the value of its last
	/// statement when `want` is set.
	fn block(&mut self, node: NodeId, want: bool) -> Result<Val<'ctx>, Error> {
		let statements = match self.view(node) {
			View::Block { statements } => statements,
			_ => {
				let value = self.expr(node)?;
				return Ok(if want { value } else { Val::void() });
			}
		};
		let count = statements.len();
		let scope = self.mark();
		let mut last = Val::void();
		for (index, statement) in statements.iter().enumerate() {
			let mark = self.mark();
			let value = self.expr(statement)?;
			if value.repr == Repr::Never {
				// what follows a `return` or a `break` is unreachable
				self.end_scope(scope, None)?;
				return Ok(Val::never());
			}
			if index + 1 == count && want {
				// the value of the block, owned by the enclosing statement
				last = self.own(value, Some(statement))?;
				self.end_statement(mark, Some(last))?;
			} else {
				self.end_statement(mark, None)?;
			}
			self.flush_after(statement)?;
		}
		self.end_scope(scope, Some(last))?;
		Ok(if want { last } else { Val::void() })
	}

	// ── expressions ─────────────────────────────────────────────────────

	/// Lowers an expression, then applies the conversion of its sink
	/// (`coercions`): the range check of a `SINK_CHECK` root, `Float32` →
	/// `Float`, `Integer` ↔ `Int64`.
	pub(crate) fn expr(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		// a read of a reactive place is tracked before it happens
		self.track_read(node)?;
		let value = self.expr_inner(node)?;
		if matches!(self.view(node), View::Call { .. }) && value.repr != Repr::Never {
			self.notify_call(node)?;
		}
		if matches!(value.repr, Repr::Void | Repr::Never) {
			return Ok(value);
		}
		match self.analysis.coercion(node) {
			Some(target) if self.flags(node).contains(NodeFlags::SINK_CHECK) => {
				self.sink_check(value, target, node)
			}
			Some(target) => {
				let repr = self.repr_of(target, node.index())?;
				match repr {
					Repr::Union | Repr::InlineUnion(_) => self.inject(value, target, node),
					Repr::Optional(values::Scalar::Union) if value.repr != Repr::None => {
						let boxed = self.inject(value, target, node)?;
						self.convert(boxed, repr, node.index())
					}
					Repr::Optional(values::Scalar::InlineUnion(_))
						if !matches!(value.repr, Repr::None | Repr::Optional(_)) =>
					{
						let inner = self.without_none(target);
						let value = self.inject(value, inner, node)?;
						self.convert(value, repr, node.index())
					}
					_ => self.convert(value, repr, node.index()),
				}
			}
			None => Ok(value),
		}
	}

	fn expr_inner(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		// a value computed at compile time (advanced brief §12): scalars and
		// strings; the other values are compiled from their expression
		if let Some(value) = self.compile_time_scalar(node)? {
			return Ok(value);
		}
		match self.view(node) {
			View::Error => Err(self.unsupported(node.index(), "code with errors")),
			View::Integer(value) => self.integer_literal(value, node),
			View::Float(value) => match self.node_repr(node)? {
				Repr::F32 => Ok(Val::new(
					Repr::F32,
					self.context.f32_type().const_float(value as f32 as f64),
				)),
				Repr::Percentage => Ok(Val::new(
					Repr::Percentage,
					self.context.f64_type().const_float(value),
				)),
				_ => Ok(Val::new(
					Repr::F64,
					self.context.f64_type().const_float(value),
				)),
			},
			View::Boolean(value) => Ok(Val::new(
				Repr::Bool,
				self.context.bool_type().const_int(value as u64, false),
			)),
			View::String(text) => Ok(self.string_constant(text)),
			View::None => Ok(Val {
				repr: Repr::None,
				value: None,
			}),
			View::Template { parts } => self.template(parts),
			View::Name(_) => self.name(node),
			View::Group(inner) => self.expr(inner),
			View::Arithmetic { op, operands } => self.arithmetic(node, op, operands),
			View::Negate(operand) => self.negate(node, operand),
			View::Percentage(operand) => self.percentage(node, operand),
			View::Equality {
				left,
				right,
				negated,
			} => self.equality(left, right, negated),
			View::Comparison {
				operands,
				operators,
			} => self.comparison(operands, operators),
			View::Logic { op, operands } => self.logic(op, operands),
			View::Not(operand) => {
				let value = self.expr(operand)?;
				let raw = value.int();
				Ok(Val::new(Repr::Bool, self.builder.build_not(raw, "")?))
			}
			View::Call { .. } => self.call(node),
			View::Access { target, members } => self.member_access(node, target, members),
			View::Object { .. } | View::TypedObject { .. } => self.object_literal(node),
			View::Array { items } => self.array_literal(node, items),
			View::Tuple { items } => self.tuple_literal(node, items),
			View::Range {
				start,
				end,
				inclusive,
			} => self.range_value(start, end, inclusive),
			View::IsIn {
				value,
				collection,
				negated,
			} => self.is_in(node, value, collection, negated),
			View::Destructuring { .. } if self.is_name_alias(node) => Ok(Val::void()),
			View::Destructuring { pattern, value, .. } => self.destructure(pattern, value),
			View::Index { target, index } => match self.index_special(node, target, index)? {
				Some(value) => Ok(value),
				None => self.index_read(node, target, index),
			},
			View::UpdateFields { target, value } => self.update_fields(target, value),
			View::Try(inner) => self.expr(inner),
			View::Lambda { .. } => self.lambda_value(node),
			View::ImplicitMember(_) => match self.analysis.resolution(node) {
				Resolution::Member(nominal, index) => match self.node_repr(node)? {
					Repr::Enum(_) => Ok(Val::new(
						self.node_repr(node)?,
						self.context.i32_type().const_int(index as u64, false),
					)),
					_ => self.build_member(nominal, index, None, node),
				},
				_ => Err(self.unsupported(node.index(), "this member")),
			},
			View::OptionalAccess { target, members } => self.optional_access(node, target, members),
			View::Is { value, ty, negated } => {
				let subject = self.expr(value)?;
				if subject.repr == Repr::Never {
					return Ok(subject);
				}
				let holds = self.is_test(subject, ty, node)?;
				let holds = if negated {
					self.builder.build_not(holds, "")?
				} else {
					holds
				};
				Ok(Val::new(Repr::Bool, holds))
			}
			View::Throw(value) => {
				let error = self.expr(value)?;
				if error.repr == Repr::Never {
					return Ok(error);
				}
				let error = self.convert(error, Repr::Union, value.index())?;
				let error = self.take(error, Some(value))?;
				self.raise(error.value.expect("an error").into_pointer_value())?;
				Ok(Val::never())
			}
			View::Catch {
				expression,
				handler,
				..
			} => self.catch_expression(node, expression, handler),
			// a `static` is a compile-time constant: nothing at runtime
			View::Declaration {
				kind: fa_analyzer::syntax::BindingKind::Static,
				..
			} => Ok(Val::void()),
			View::Declaration {
				kind: fa_analyzer::syntax::BindingKind::Reactive,
				value,
				..
			} => self.reactive_declaration(node, value),
			View::Declaration {
				kind: fa_analyzer::syntax::BindingKind::Derived,
				value,
				..
			} => self.derived_declaration(node, value),
			View::Declaration { value, .. } => self.declaration(node, value),
			View::Watch(_) => self.watch(node),
			View::ExportValue { value, .. } => self.declaration(node, Some(value)),
			View::ExportFunction { .. } | View::ExportType { .. } => Ok(Val::void()),
			// the members of a namespace of a module are initialized in the
			// program's order; a program file's, where it is declared
			View::Namespace { value, .. } | View::ExportNamespace(value) => {
				if self.module_file {
					Ok(Val::void())
				} else {
					self.namespace_statement(value)
				}
			}
			View::Use { .. } if self.is_name_alias(node) => Ok(Val::void()),
			View::Use { value, .. } => self.use_declaration(node, value),
			View::Provide { value, .. } => self.provide(node, value),
			View::Reuse { value, .. } => self.reuse(node, value),
			View::Assignment { target, value, .. } => {
				let Some(value) = value else {
					return Err(self.unsupported(node.index(), "an assignment without a value"));
				};
				if self.analysis.relation_writes.contains_key(&node) {
					return self.relation_assignment(node, target, value);
				}
				self.reactive_write(node, target, |this| this.assignment(target, value))
			}
			View::CompoundAssignment { op, target, value } => {
				self.reactive_write(node, target, |this| {
					this.compound_assignment(node, op, target, value)
				})
			}
			View::Function { .. } | View::TypeDecl { .. } => Ok(Val::void()),
			View::If {
				condition,
				then_body,
				else_body,
				compile_time: false,
			} => self.if_expression(node, condition, then_body, else_body),
			// `@if`: the branch the analysis kept (D22)
			View::If {
				then_body,
				else_body,
				compile_time: true,
				..
			} => match self.comptime_branch(node) {
				Some(true) => self.expr(then_body),
				Some(false) => match else_body {
					Some(else_body) => self.expr(else_body),
					None => Ok(Val::void()),
				},
				None => Ok(Val::void()),
			},
			View::While { condition, body } => self.while_loop(node, condition, body),
			View::WhileLet {
				pattern,
				value,
				body,
			} => self.while_let_loop(node, pattern, value, body),
			View::For {
				pattern,
				index,
				iterable,
				body,
				parallel: None,
				compile_time: false,
			} => self.for_loop(node, pattern, index.is_some(), iterable, body),
			View::For {
				pattern,
				index,
				iterable,
				body,
				parallel: None,
				compile_time: true,
			} => self.comptime_for(node, pattern, index.is_some(), iterable, body),
			// `for parallel` (parallel.rs)
			View::For {
				pattern,
				index,
				iterable,
				body,
				parallel: Some(options),
				compile_time: false,
			} => self.parallel_for(node, pattern, index.is_some(), iterable, body, options),
			View::Loop(body) => self.loop_expression(node, body),
			View::Fallback { operands } => self.fallback(node, operands),
			View::When { .. } => self.when_expression(node),
			View::Do(body) => {
				let want = self.node_repr(node)?.has_value();
				self.block(body, want)
			}
			View::Block { .. } => {
				let want = self.node_repr(node).map(Repr::has_value).unwrap_or(false);
				self.block(node, want)
			}
			View::Break(value) => self.break_statement(node, value),
			View::Continue => {
				let next = self
					.frame()
					.loops
					.last()
					.map(|frame| (frame.next, frame.mark));
				let Some((next, mark)) = next else {
					return Err(self.unsupported(node.index(), "`continue` outside a loop"));
				};
				self.drop_to(mark)?;
				self.builder.build_unconditional_branch(next)?;
				self.continue_unreachable();
				Ok(Val::never())
			}
			View::Return(value) => {
				let result = match value {
					Some(value) => self.expr(value)?,
					None => Val::void(),
				};
				self.build_return(result, value.unwrap_or(node))?;
				self.continue_unreachable();
				Ok(Val::never())
			}
			View::Assert(condition) => self.assert(node, condition),
			View::Unsupported(name) => Err(self.unsupported(node.index(), name)),
			// design/async.md §3 (`tasks.rs`)
			View::Await(operand) => self.await_expression(node, operand),
			View::Start(operand) => self.start_expression(node, operand),
			other => Err(self.unsupported(node.index(), &format!("`{}`", variant_name(&other)))),
		}
	}

	/// `value is T`: a type, a range (`score is 0..60`), or alternatives of
	/// them (`value is 1 | 2 | 10..`).
	fn is_test(
		&mut self,
		subject: Val<'ctx>,
		ty: NodeId,
		node: NodeId,
	) -> Result<IntValue<'ctx>, Error> {
		match self.view(ty) {
			View::Range {
				start,
				end,
				inclusive,
			} => self.range_test(subject, start, end, inclusive, node),
			View::Group(inner) => self.is_test(subject, inner, node),
			View::Union { operands } if subject.repr != Repr::Union => {
				let mut result = self.context.bool_type().const_zero();
				for operand in operands.iter() {
					let holds = self.is_test(subject, operand, node)?;
					result = self.builder.build_or(result, holds, "")?;
				}
				Ok(result)
			}
			_ => {
				let tested = self.type_of(ty);
				self.type_test(subject, tested, node)
			}
		}
	}

	/// A name: a local, `arguments`, or a function used as a value (unsupported).
	fn name(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		match self.analysis.resolution(node) {
			Resolution::Local(symbol) => {
				let kind = self.analysis.symbols.symbol(symbol).kind;
				if matches!(kind, SymbolKind::Function) {
					return self.function_of_symbol(symbol, node);
				}
				if matches!(kind, SymbolKind::Derived) {
					return self.derived_read(symbol, node);
				}
				let slot = self.slot_of(symbol, node)?;
				if self.nullable_references.contains_key(&symbol)
					&& matches!(self.node_repr(node), Ok(Repr::Optional(_)))
				{
					return self.optional_element(slot.pointer, slot.repr, node);
				}
				let value = self.load(slot)?;
				let narrowed = self.node_repr(node).unwrap_or(value.repr);
				self.narrow_to(value, narrowed, node)
			}
			Resolution::External(..) => self.external_name(node),
			Resolution::Builtin(id) if fa_analyzer::builtins::get(id).path == "arguments" => {
				let arguments = self.runtime.function("fa_arguments")?;
				let array = self.call_value(arguments, &[])?;
				Ok(self.fresh(Val::new(Repr::Array(TypeId::STRING), array)))
			}
			_ => Err(self.unsupported(node.index(), "this name")),
		}
	}

	/// `let x = value`, `mutable x: T = value`.
	fn declaration(&mut self, node: NodeId, value: Option<NodeId>) -> Result<Val<'ctx>, Error> {
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(node) else {
			return Err(self.unsupported(node.index(), "this declaration"));
		};
		let Some(value_node) = value else {
			return Err(self.unsupported(node.index(), "a declaration without a value"));
		};
		// an iterator is no value: its chain runs where it is consumed
		if self.is_iterator(value_node) {
			return Ok(Val::void());
		}
		// `let codes = word.codePoints()` only indexed and measured: no array
		if self.virtual_code_points_of(node, symbol, value_node) {
			self.virtual_code_points.insert(symbol, value_node);
			return Ok(Val::void());
		}
		if self.shared.contains_key(&symbol) && self.immutable_scalar(symbol)?.is_some() {
			self.slot_of(symbol, node)?;
			return Ok(Val::void());
		}
		let value = self.expr(value_node)?;
		if value.repr == Repr::Never {
			return Ok(Val::never());
		}
		let slot = self.slot_of(symbol, node)?;
		let value =
			if matches!(slot.repr, Repr::Union | Repr::InlineUnion(_)) && value.repr != slot.repr {
				let ty = self.analysis.symbols.symbol(symbol).ty;
				self.inject(value, ty, value_node)?
			} else {
				value
			};
		let value = self.owned_as(value, slot.repr, Some(value_node))?;
		self.store(slot, value, value_node)?;
		self.own_local(symbol, slot);
		self.prepare_reciprocal(symbol, value)?;
		Ok(Val::void())
	}

	/// `target = value`: a local, a field.
	fn assignment(&mut self, target: NodeId, value: NodeId) -> Result<Val<'ctx>, Error> {
		if let Resolution::Local(symbol) = self.analysis.resolution(target)
			&& self.analysis.symbols.symbol(symbol).kind == SymbolKind::Mutable
			&& self.node_repr(target)? == Repr::Str
			&& let View::Template { parts } = self.view(value)
		{
			let slot = self.place(target)?;
			if let Some(result) = self.string_template(parts, Some(slot))? {
				return Ok(result);
			}
		}
		let value_lowered = self.expr(value)?;
		if value_lowered.repr == Repr::Never {
			return Ok(Val::never());
		}
		// an array read from a place into an array local: its items are
		// copied into the local's own block (no allocation, nothing leaked)
		if let (View::Name(_), Repr::Array(item_type)) = (self.view(target), value_lowered.repr)
			&& self.is_place_read(value)
			&& self.copies_bytewise(item_type, value)?
		{
			let slot = self.place(target)?;
			let destination = self.load(slot)?;
			let item = self.item_repr(item_type, value)?;
			let size =
				inkwell::types::BasicType::size_of(&self.llvm_type(item)).expect("a sized item");
			let size = self
				.builder
				.build_int_cast(size, self.context.i64_type(), "")?;
			self.call_runtime(
				"fa_array_assign",
				&[
					destination.value.expect("an array").into(),
					value_lowered.value.expect("an array").into(),
					size.into(),
				],
			)?;
			return Ok(Val::void());
		}
		let slot = self.place(target)?;
		if slot.repr == Repr::Str
			&& self
				.constant_text(value)
				.is_some_and(|text| text.is_empty())
		{
			self.call_runtime(
				"fa_string_replace_buffer",
				&[
					self.usize.const_zero().into(),
					self.simple_flag().into(),
					slot.pointer.into(),
				],
			)?;
			return Ok(Val::void());
		}
		let value_lowered = self.owned_as(value_lowered, slot.repr, Some(value))?;
		// the value it replaces is dropped
		let old = self.load(slot)?;
		self.emit_drop(old)?;
		self.store(slot, value_lowered, value)?;
		Ok(Val::void())
	}

	// ── control flow ────────────────────────────────────────────────────

	/// The condition of an `if`, a `while` or an `assert`: an `i1`.
	fn condition(&mut self, node: NodeId) -> Result<IntValue<'ctx>, Error> {
		let value = self.region(None, node, |this| this.expr(node))?;
		match value.repr {
			Repr::Bool => Ok(value.int()),
			Repr::Never => Ok(self.context.bool_type().const_zero()),
			_ => Err(self.unsupported(node.index(), "a condition that isn't a Boolean")),
		}
	}

	fn if_expression(
		&mut self,
		node: NodeId,
		condition: NodeId,
		then_body: NodeId,
		else_body: Option<NodeId>,
	) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node).unwrap_or(Repr::Void);
		let slot = if repr.has_value() && else_body.is_some() {
			Some(self.alloca(repr, "if")?)
		} else {
			None
		};
		let condition = self.condition(condition)?;
		let then_block = self.append_block("then");
		let else_block = self.append_block("else");
		let join = self.append_block("end_if");
		self.builder
			.build_conditional_branch(condition, then_block, else_block)?;
		let mut reached = false;
		for (block, body) in [(then_block, Some(then_body)), (else_block, else_body)] {
			self.builder.position_at_end(block);
			if let Some(body) = body {
				let owned = slot.is_some();
				let value = self.region(slot.map(|slot| slot.repr), body, |this| {
					match this.view(body) {
						View::If { .. } => this.expr(body),
						_ => this.block(body, owned),
					}
				})?;
				if value.repr == Repr::Never {
					continue;
				}
				if let Some(slot) = slot {
					self.store(slot, value, body)?;
				}
			}
			reached = true;
			self.branch_to(join)?;
		}
		self.builder.position_at_end(join);
		if !reached {
			self.builder.build_unreachable()?;
			self.continue_unreachable();
			return Ok(Val::never());
		}
		match slot {
			Some(slot) => Ok(self.fresh(self.load(slot)?)),
			None => Ok(Val::void()),
		}
	}

	fn while_loop(
		&mut self,
		_node: NodeId,
		condition: NodeId,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let test = self.append_block("while");
		let body_block = self.append_block("while_body");
		let exit = self.append_block("end_while");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let condition = self.condition(condition)?;
		self.builder
			.build_conditional_branch(condition, body_block, exit)?;
		self.builder.position_at_end(body_block);
		let mark = self.mark();
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next: test,
			value: None,
			mark,
		});
		self.coop_point()?;
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.branch_to(test)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}

	/// `while let name = value { … }`: the value read before each round, the
	/// loop left at the first `none`. Optional scalars for now.
	fn while_let_loop(
		&mut self,
		node: NodeId,
		pattern: NodeId,
		value: NodeId,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let test = self.append_block("while_let");
		let exit = self.append_block("end_while_let");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let mark = self.mark();
		let current = self.expr(value)?;
		if current.repr == Repr::Never {
			return Ok(current);
		}
		if !matches!(current.repr, Repr::Optional(_)) {
			return Err(self.unsupported(node.index(), "`while let` over this value"));
		}
		let present = self.present_or_exit(current, exit)?;
		if let Some(symbol) = self.analysis.symbols.symbol_of_decl(pattern) {
			let slot = self.slot_of(symbol, node)?;
			self.store(slot, present, node)?;
		}
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next: test,
			value: None,
			mark,
		});
		self.coop_point()?;
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.end_scope(mark, None)?;
		self.branch_to(test)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}

	/// `for i in a..b`, `for i in a..=b`, `for i in a..`.
	fn for_loop(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let View::Range {
			start: Some(start),
			end,
			inclusive,
		} = self.view(iterable)
		else {
			if self.is_iterator(iterable) {
				return self.for_iterator(node, pattern, iterable, body);
			}
			if let Some(code_points) = self.code_points_call(iterable) {
				let text = self.receiver(code_points)?;
				if text.repr == Repr::Never {
					return Ok(text);
				}
				let bytes = self.is_bytes_call(code_points);
				return self.for_code_points(node, pattern, has_index, text, body, bytes);
			}
			let value = self.expr(iterable)?;
			return match value.repr {
				Repr::Str => self.for_characters(node, pattern, has_index, value, body),
				Repr::Array(_) | Repr::Growing(_) => {
					self.for_items(node, pattern, has_index, value, body)
				}
				Repr::Map(_) | Repr::Set(_) => self.for_entries(node, pattern, value, body),
				Repr::Pool(_) | Repr::Columnar(_) => self.for_pool(node, pattern, value, body),
				Repr::Links(_) => self.for_links(node, pattern, value, body),
				Repr::Range => self.for_range_value(node, pattern, value, body),
				Repr::Never => Ok(value),
				_ => Err(self.unsupported(iterable.index(), "`for` over this value")),
			};
		};
		let i64 = self.context.i64_type();
		let start = self.expr(start)?;
		let start = self.convert(start, Repr::Int(IntRepr::I64), iterable.index())?;
		let open_end = end.is_none();
		let (end, inclusive) = match end {
			Some(end) => {
				let end = self.expr(end)?;
				let end = self.convert(end, Repr::Int(IntRepr::I64), iterable.index())?;
				(end.int(), inclusive)
			}
			None => (i64.const_int(i64::MAX as u64, true), true),
		};
		let reserved =
			self.reserve_for_pushes(body, start.int(), (!open_end).then_some(end), inclusive)?;
		let counter =
			match pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern)) {
				Some(symbol) => self.slot_of(symbol, node)?,
				None => self.alloca(Repr::Int(IntRepr::I64), "counter")?,
			};
		self.builder.build_store(counter.pointer, start.int())?;
		let appends = if open_end {
			Vec::new()
		} else {
			self.reserve_for_appends(body, start.int(), end, inclusive, counter.pointer)?
		};
		let test = self.append_block("for");
		let body_block = self.append_block("for_body");
		let step = self.append_block("for_step");
		let exit = self.append_block("end_for");
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(test);
		let current = self.load(counter)?.int();
		let predicate = if inclusive {
			IntPredicate::SLE
		} else {
			IntPredicate::SLT
		};
		let more = self
			.builder
			.build_int_compare(predicate, current, end, "")?;
		self.builder
			.build_conditional_branch(more, body_block, exit)?;
		self.builder.position_at_end(body_block);
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
		self.branch_to(step)?;
		self.builder.position_at_end(step);
		let current = self.load(counter)?.int();
		if inclusive {
			// `a..=Integer.maxValue` ends without computing `maxValue + 1`
			let last = self
				.builder
				.build_int_compare(IntPredicate::EQ, current, end, "")?;
			let increment = self.append_block("for_increment");
			self.builder
				.build_conditional_branch(last, exit, increment)?;
			self.builder.position_at_end(increment);
		}
		let next = self
			.builder
			.build_int_nsw_add(current, i64.const_int(1, false), "")?;
		self.builder.build_store(counter.pointer, next)?;
		self.builder.build_unconditional_branch(test)?;
		self.builder.position_at_end(exit);
		for push in reserved {
			self.reserved_pushes.remove(&push);
		}
		for append in appends {
			self.reserved_appends.remove(&append);
		}
		Ok(Val::void())
	}

	/// `loop { … }`, whose value is the value of its `break`.
	fn loop_expression(&mut self, node: NodeId, body: NodeId) -> Result<Val<'ctx>, Error> {
		let repr = self.node_repr(node).unwrap_or(Repr::Void);
		let value = if repr.has_value() {
			Some(self.alloca(repr, "loop")?)
		} else {
			None
		};
		let body_block = self.append_block("loop");
		let exit = self.append_block("end_loop");
		self.builder.build_unconditional_branch(body_block)?;
		self.builder.position_at_end(body_block);
		let mark = self.mark();
		self.frame_mut().loops.push(LoopFrame {
			exit,
			next: body_block,
			value,
			mark,
		});
		self.coop_point()?;
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.branch_to(body_block)?;
		self.builder.position_at_end(exit);
		if repr == Repr::Never {
			self.builder.build_unreachable()?;
			self.continue_unreachable();
			return Ok(Val::never());
		}
		match value {
			Some(slot) => Ok(self.fresh(self.load(slot)?)),
			None => Ok(Val::void()),
		}
	}

	fn break_statement(&mut self, node: NodeId, value: Option<NodeId>) -> Result<Val<'ctx>, Error> {
		let Some((exit, slot, mark)) = self
			.frame()
			.loops
			.last()
			.map(|frame| (frame.exit, frame.value, frame.mark))
		else {
			return Err(self.unsupported(node.index(), "`break` outside a loop"));
		};
		if let Some(value) = value {
			let lowered = self.expr(value)?;
			if lowered.repr == Repr::Never {
				return Ok(Val::never());
			}
			if let Some(slot) = slot {
				let lowered = self.owned_as(lowered, slot.repr, Some(value))?;
				self.store(slot, lowered, value)?;
			}
		}
		self.drop_to(mark)?;
		self.builder.build_unconditional_branch(exit)?;
		self.continue_unreachable();
		Ok(Val::never())
	}

	/// `assert condition`: a stop with the source of the condition when false.
	fn assert(&mut self, node: NodeId, condition: NodeId) -> Result<Val<'ctx>, Error> {
		// a comparison of two operands: they are printed when it fails
		let operands = match self.view(condition) {
			View::Equality {
				left,
				right,
				negated,
			} => Some((left, right, None, negated)),
			View::Comparison {
				operands,
				operators,
			} if operands.len() == 2 => Some((
				operands.get(0).expect("an operand"),
				operands.get(1).expect("an operand"),
				operators.get(0),
				false,
			)),
			_ => None,
		};
		if let Some((left, right, operator, negated)) = operands {
			let a = self.expr(left)?;
			let b = self.expr(right)?;
			if a.repr == Repr::Never || b.repr == Repr::Never {
				return Ok(Val::never());
			}
			let holds = match operator {
				Some(operator) => {
					let predicate = match operator {
						syntax::CompareOp::Less => IntPredicate::SLT,
						_ => IntPredicate::SLE,
					};
					// `>` and `>=` swap their operands
					match operator {
						syntax::CompareOp::Less | syntax::CompareOp::LessOrEqual => {
							self.values_order(a, b, predicate, condition)?
						}
						syntax::CompareOp::Greater => {
							self.values_order(b, a, IntPredicate::SLT, condition)?
						}
						syntax::CompareOp::GreaterOrEqual => {
							self.values_order(b, a, IntPredicate::SLE, condition)?
						}
					}
				}
				None => {
					let equal = self.values_equal_any(a, b, condition)?;
					if negated {
						self.builder.build_not(equal, "")?
					} else {
						equal
					}
				}
			};
			let failed = self.builder.build_not(holds, "")?;
			let source = self.tree.text(condition.index()).to_string();
			let site = self.site(&source, node);
			self.guard(failed, |this| {
				let mark = this.mark();
				this.call_runtime("fa_b_reset", &[])?;
				let text = this.string_constant("left: ");
				this.append_value(text, node)?;
				this.append_nested(a, node)?;
				let text = this.string_constant(", right: ");
				this.append_value(text, node)?;
				this.append_nested(b, node)?;
				this.end_scope(mark, None)?;
				this.call_noreturn("fa_assert_failed_operands", &[site.into()])
			})?;
			return Ok(Val::void());
		}
		let value = self.condition(condition)?;
		let failed = self.builder.build_not(value, "")?;
		let source = self.tree.text(condition.index()).to_string();
		let site = self.site(&source, node);
		self.guard(failed, |this| {
			this.call_noreturn("fa_assert_failed", &[site.into()])
		})?;
		Ok(Val::void())
	}
}

/// The zero value of an IR type, for the initializer of a global.
pub(crate) fn zero_of(ty: BasicTypeEnum<'_>) -> inkwell::values::BasicValueEnum<'_> {
	ty.const_zero()
}

/// A short name of a view variant, for messages.
fn variant_name(view: &View<'_>) -> String {
	let text = format!("{view:?}");
	text.split([' ', '(', '{'])
		.next()
		.unwrap_or("this construct")
		.to_string()
}
