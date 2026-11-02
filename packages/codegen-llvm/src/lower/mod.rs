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

mod arith;
mod calls;
mod patterns;
mod text;
pub(crate) mod values;

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
const FAST_CALLING_CONVENTION: u32 = 8;

/// A lowered function declaration.
struct Function<'ctx> {
	value: FunctionValue<'ctx>,
	info: FnInfo,
	/// The representation of each parameter, and whether it has a default
	/// (then an `i1` follows it).
	params: Vec<(Repr, bool)>,
	ret: Repr,
}

/// The storage of a local: a pointer (an alloca or a global) and its representation.
#[derive(Copy, Clone)]
struct Slot<'ctx> {
	pointer: PointerValue<'ctx>,
	repr: Repr,
}

/// A loop being lowered: where `break` and `continue` go.
struct LoopFrame<'ctx> {
	node: NodeId,
	exit: BasicBlock<'ctx>,
	next: BasicBlock<'ctx>,
	/// The slot of the value of a `loop` used as a value.
	value: Option<Slot<'ctx>>,
}

/// The function being lowered.
struct Frame<'ctx> {
	function: FunctionValue<'ctx>,
	/// The block holding the allocas, before `start`.
	allocas: BasicBlock<'ctx>,
	/// The return representation (`Void` for `main`).
	ret: Repr,
	/// The function declaration, `None` for the top level.
	owner: Option<NodeId>,
	loops: Vec<LoopFrame<'ctx>>,
}

/// Lowers one analyzed file into an LLVM module.
pub(crate) struct Lowering<'a, 'ctx> {
	session: &'a Session,
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
	pair_type: StructType<'ctx>,
	/// The function declarations, by their `function` node.
	functions: FxHashMap<NodeId, Function<'ctx>>,
	/// The storage of every local reached so far.
	slots: FxHashMap<SymbolId, Slot<'ctx>>,
	/// The function declaration owning each scope (`None`: the top level).
	scope_owners: FxHashMap<ScopeId, Option<NodeId>>,
	/// The top-level locals that functions read: globals.
	shared: FxHashMap<SymbolId, ()>,
	frame: Option<Frame<'ctx>>,
	/// The constants already emitted: string literals and check sites.
	constants: FxHashMap<Vec<u8>, PointerValue<'ctx>>,
	/// Line starts of the source, for locations.
	lines: Vec<usize>,
}

impl<'a, 'ctx> Lowering<'a, 'ctx> {
	pub(crate) fn new(
		session: &'a Session,
		file: FileId,
		path: &str,
		context: &'ctx Context,
		module: &'a Module<'ctx>,
		target: Target,
		usize: IntType<'ctx>,
	) -> Self {
		let tree = session.tree(file);
		let pointer = context.ptr_type(AddressSpace::default());
		let lines = std::iter::once(0)
			.chain(
				tree.source
					.bytes()
					.enumerate()
					.filter(|(_, byte)| *byte == b'\n')
					.map(|(index, _)| index + 1),
			)
			.collect();
		Lowering {
			session,
			tree,
			analysis: session.analysis(file),
			path: path.to_string(),
			context,
			module,
			builder: context.create_builder(),
			runtime: Runtime::new(context, module, target, usize),
			target,
			usize,
			pair_type: context.struct_type(&[pointer.into(), usize.into()], false),
			functions: FxHashMap::default(),
			slots: FxHashMap::default(),
			scope_owners: FxHashMap::default(),
			shared: FxHashMap::default(),
			frame: None,
			constants: FxHashMap::default(),
			lines,
		}
	}

	pub(crate) fn types(&self) -> &'a TypeStore {
		self.session.types()
	}

	/// Lowers the whole file: the runtime, the functions, then the top level.
	pub(crate) fn lower(mut self) -> Result<(), Error> {
		self.runtime.link_tier1()?;
		self.index_functions()?;
		self.find_shared_locals();
		let mut declarations: Vec<NodeId> = self.functions.keys().copied().collect();
		declarations.sort();
		for declaration in declarations {
			self.lower_function(declaration)?;
		}
		self.lower_main()?;
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

	pub(crate) fn flags(&self, node: NodeId) -> NodeFlags {
		self.analysis.flags_of(node)
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
		let value = self
			.pair_type
			.const_named_struct(&[pointer.into(), length.into()]);
		Val::new(Repr::Str, value)
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
		let value = self
			.builder
			.build_load(self.llvm_type(slot.repr), slot.pointer, "")?;
		Ok(Val::new(slot.repr, value))
	}

	fn store(&self, slot: Slot<'ctx>, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		let value = self.convert(value, slot.repr, node.index())?;
		if let Some(raw) = value.value {
			self.builder.build_store(slot.pointer, raw)?;
		}
		Ok(())
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
		let i32 = self.context.i32_type();
		let weights = self.context.metadata_node(&[
			self.context.metadata_string("branch_weights").into(),
			i32.const_int(1, false).into(),
			i32.const_int(1 << 20, false).into(),
		]);
		branch
			.set_metadata(weights, self.context.get_kind_id("prof"))
			.map_err(|error| Error::Llvm(error.to_string()))?;
		self.builder.position_at_end(fail);
		cold(self)?;
		self.builder.position_at_end(pass);
		Ok(())
	}

	// ── functions ───────────────────────────────────────────────────────

	/// Declares every function declaration of the file.
	fn index_functions(&mut self) -> Result<(), Error> {
		let mut infos: Vec<(NodeId, FnInfo)> = self
			.analysis
			.functions
			.iter()
			.filter(|(node, info)| info.declaration == Some(**node))
			.map(|(node, info)| (*node, info.clone()))
			.collect();
		infos.sort_by_key(|(node, _)| *node);
		for (node, info) in infos {
			self.scope_owners.insert(info.scope, Some(node));
			let Some(sig) = info.sig else {
				return Err(self.unsupported(node.index(), "a function without a signature"));
			};
			let sig = self.types().sig(sig);
			let ret = self.repr_of(sig.ret, node.index())?;
			let mut params = Vec::new();
			let mut types: Vec<BasicMetadataTypeEnum> = Vec::new();
			for param in &info.params {
				if param.variadic {
					return Err(self.unsupported(param.node.index(), "a variadic parameter"));
				}
				let ty = self.analysis.symbols.symbol(param.symbol).ty;
				let repr = self.repr_of(ty, param.node.index())?;
				if !repr.has_value() {
					return Err(self.unsupported(param.node.index(), "this parameter type"));
				}
				types.push(self.llvm_type(repr).into());
				if param.default.is_some() {
					types.push(self.context.bool_type().into());
				}
				params.push((repr, param.default.is_some()));
			}
			let fn_type = if ret.has_value() {
				self.llvm_type(ret).fn_type(&types, false)
			} else {
				self.context.void_type().fn_type(&types, false)
			};
			let name = info
				.symbol
				.map(|symbol| {
					let symbol = self.analysis.symbols.symbol(symbol);
					self.session.types().syms.resolve(symbol.name).to_string()
				})
				.unwrap_or_else(|| "lambda".to_string());
			let value =
				self.module
					.add_function(&format!("fa.{name}"), fn_type, Some(Linkage::Internal));
			value.set_call_conventions(FAST_CALLING_CONVENTION);
			self.runtime.add_attributes(value, &["nounwind"]);
			for index in 0..types.len() {
				value.add_attribute(
					AttributeLoc::Param(index as u32),
					self.runtime.attribute("noundef"),
				);
			}
			if ret == Repr::Never {
				self.runtime.add_attributes(value, &["noreturn"]);
			}
			self.functions.insert(
				node,
				Function {
					value,
					info,
					params,
					ret,
				},
			);
		}
		Ok(())
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
	}

	/// Starts lowering a function: its allocas block and its first block.
	fn begin_function(&mut self, function: FunctionValue<'ctx>, ret: Repr, owner: Option<NodeId>) {
		let allocas = self.context.append_basic_block(function, "allocas");
		let start = self.context.append_basic_block(function, "start");
		self.builder.position_at_end(start);
		self.frame = Some(Frame {
			function,
			allocas,
			ret,
			owner,
			loops: Vec::new(),
		});
	}

	/// Ends the function: the allocas block falls into the first block.
	fn end_function(&mut self) -> Result<(), Error> {
		let frame = self.frame.take().expect("inside a function");
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

	fn lower_function(&mut self, declaration: NodeId) -> Result<(), Error> {
		let function = &self.functions[&declaration];
		let (value, info, params, ret) = (
			function.value,
			function.info.clone(),
			function.params.clone(),
			function.ret,
		);
		self.begin_function(value, ret, Some(declaration));
		// the parameters, then their defaults in order
		let mut index = 0;
		for (param, (repr, has_default)) in info.params.iter().zip(params) {
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
					self.builder
						.build_conditional_branch(flag, join, use_default)?;
					self.builder.position_at_end(use_default);
					let default_value = self.expr(default)?;
					self.store(slot, default_value, default)?;
					self.branch_to(join)?;
					self.builder.position_at_end(join);
				}
				_ => {
					self.builder.build_store(slot.pointer, given)?;
				}
			}
		}
		let result = if info.block_body {
			self.block(info.body, ret.has_value())?
		} else {
			self.expr(info.body)?
		};
		self.build_return(result, info.body)?;
		self.end_function()
	}

	/// Returns from the current function with a value converted to its result.
	fn build_return(&mut self, value: Val<'ctx>, node: NodeId) -> Result<(), Error> {
		if value.repr == Repr::Never {
			self.builder.build_unreachable()?;
			return Ok(());
		}
		let ret = self.frame().ret;
		if self.frame().owner.is_none() {
			// the top level: flush and exit normally
			self.call_runtime("fa_flush", &[])?;
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
				Some(raw) => self.builder.build_return(Some(&raw))?,
				None => self.builder.build_unreachable()?,
			};
		} else {
			self.builder.build_return(None)?;
		}
		Ok(())
	}

	/// `main(argc, argv)`, or `_start()` on wasm: the top level of the file.
	fn lower_main(&mut self) -> Result<(), Error> {
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
		self.begin_function(function, Repr::Void, None);
		if self.target == Target::Native && !self.functions.is_empty() {
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
		let root = NodeId::new(self.tree.root);
		let View::Module { statements } = self.view(root) else {
			return Err(self.unsupported(self.tree.root, "this file"));
		};
		for statement in statements.iter() {
			self.statement(statement)?;
		}
		self.build_return(Val::void(), root)?;
		self.end_function()
	}

	/// Whether the program reads `arguments`.
	fn uses_arguments(&self) -> bool {
		self.analysis.resolutions.iter().any(|resolution| {
			matches!(resolution, Resolution::Builtin(id)
				if fa_analyzer::builtins::get(*id).path == "arguments")
		})
	}

	// ── locals ──────────────────────────────────────────────────────────

	/// The slot of a local, created on first use: an alloca of the function
	/// owning it, or a global for a top-level local that functions read.
	fn slot_of(&mut self, symbol: SymbolId, node: NodeId) -> Result<Slot<'ctx>, Error> {
		if let Some(slot) = self.slots.get(&symbol) {
			return Ok(*slot);
		}
		let data = self.analysis.symbols.symbol(symbol);
		let repr = self.repr_of(data.ty, node.index())?;
		if !repr.has_value() {
			return Err(self.unsupported(node.index(), "a local without a value"));
		}
		let owner = self.owner_of_scope(data.scope);
		let slot = if self.shared.contains_key(&symbol) {
			let ty = self.llvm_type(repr);
			let global = self.module.add_global(ty, None, "fa.global");
			global.set_linkage(Linkage::Internal);
			global.set_initializer(&zero_of(ty));
			Slot {
				pointer: global.as_pointer_value(),
				repr,
			}
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
		let mut last = Val::void();
		for (index, statement) in statements.iter().enumerate() {
			let value = self.expr(statement)?;
			if index + 1 == count {
				last = value;
			}
			if value.repr == Repr::Never {
				// what follows a `return` or a `break` is unreachable
				return Ok(Val::never());
			}
		}
		Ok(if want { last } else { Val::void() })
	}

	// ── expressions ─────────────────────────────────────────────────────

	/// Lowers an expression, then applies the conversion of its sink
	/// (`coercions`): the range check of a `SINK_CHECK` root, `Float32` →
	/// `Float`, `Integer` ↔ `Int64`.
	pub(crate) fn expr(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		let value = self.expr_inner(node)?;
		if matches!(value.repr, Repr::Void | Repr::Never) {
			return Ok(value);
		}
		match self.analysis.coercion(node) {
			Some(target) if self.flags(node).contains(NodeFlags::SINK_CHECK) => {
				self.sink_check(value, target, node)
			}
			Some(target) => {
				let repr = self.repr_of(target, node.index())?;
				self.convert(value, repr, node.index())
			}
			None => Ok(value),
		}
	}

	fn expr_inner(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
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
			View::Access { target, members } => self.access(node, target, members),
			View::Declaration { value, .. } => self.declaration(node, value),
			View::Assignment { target, value, .. } => {
				let Some(value) = value else {
					return Err(self.unsupported(node.index(), "an assignment without a value"));
				};
				self.assignment(target, value)
			}
			View::CompoundAssignment { op, target, value } => {
				self.compound_assignment(node, op, target, value)
			}
			View::Function { .. } | View::TypeDecl { .. } => Ok(Val::void()),
			View::If {
				condition,
				then_body,
				else_body,
				compile_time: false,
			} => self.if_expression(node, condition, then_body, else_body),
			View::While { condition, body } => self.while_loop(node, condition, body),
			View::For {
				pattern,
				index: None,
				iterable,
				body,
				parallel: None,
				compile_time: false,
			} => self.for_loop(node, pattern, iterable, body),
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
				let next = self.frame().loops.last().map(|frame| frame.next);
				let Some(next) = next else {
					return Err(self.unsupported(node.index(), "`continue` outside a loop"));
				};
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
			other => Err(self.unsupported(node.index(), &format!("`{}`", variant_name(&other)))),
		}
	}

	/// A name: a local, `arguments`, or a function used as a value (unsupported).
	fn name(&mut self, node: NodeId) -> Result<Val<'ctx>, Error> {
		match self.analysis.resolution(node) {
			Resolution::Local(symbol) => {
				let kind = self.analysis.symbols.symbol(symbol).kind;
				if matches!(kind, SymbolKind::Function) {
					return Err(self.unsupported(node.index(), "a function used as a value"));
				}
				let slot = self.slot_of(symbol, node)?;
				let value = self.load(slot)?;
				self.narrow(value, node)
			}
			Resolution::Builtin(id) if fa_analyzer::builtins::get(id).path == "arguments" => {
				let out = self.alloca(Repr::StrArray, "arguments")?;
				self.call_runtime("fa_arguments", &[out.pointer.into()])?;
				self.load(out)
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
		let value = self.expr(value_node)?;
		if value.repr == Repr::Never {
			return Ok(Val::never());
		}
		let slot = self.slot_of(symbol, node)?;
		self.store(slot, value, value_node)?;
		Ok(Val::void())
	}

	/// `target = value` on a local.
	fn assignment(&mut self, target: NodeId, value: NodeId) -> Result<Val<'ctx>, Error> {
		let Resolution::Local(symbol) = self.analysis.resolution(target) else {
			return Err(self.unsupported(target.index(), "this assignment target"));
		};
		let value_lowered = self.expr(value)?;
		if value_lowered.repr == Repr::Never {
			return Ok(Val::never());
		}
		let slot = self.slot_of(symbol, target)?;
		self.store(slot, value_lowered, value)?;
		Ok(Val::void())
	}

	// ── control flow ────────────────────────────────────────────────────

	/// The condition of an `if`, a `while` or an `assert`: an `i1`.
	fn condition(&mut self, node: NodeId) -> Result<IntValue<'ctx>, Error> {
		let value = self.expr(node)?;
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
				let value = match self.view(body) {
					View::If { .. } => self.expr(body)?,
					_ => self.block(body, slot.is_some())?,
				};
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
			Some(slot) => self.load(slot),
			None => Ok(Val::void()),
		}
	}

	fn while_loop(
		&mut self,
		node: NodeId,
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
		self.frame_mut().loops.push(LoopFrame {
			node,
			exit,
			next: test,
			value: None,
		});
		self.block(body, false)?;
		self.frame_mut().loops.pop();
		self.branch_to(test)?;
		self.builder.position_at_end(exit);
		Ok(Val::void())
	}

	/// `for i in a..b`, `for i in a..=b`, `for i in a..`.
	fn for_loop(
		&mut self,
		node: NodeId,
		pattern: Option<NodeId>,
		iterable: NodeId,
		body: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let View::Range {
			start: Some(start),
			end,
			inclusive,
		} = self.view(iterable)
		else {
			let value = self.expr(iterable)?;
			return match value.repr {
				Repr::Str => self.for_characters(node, pattern, value, body),
				Repr::Never => Ok(value),
				_ => Err(self.unsupported(iterable.index(), "`for` over this value")),
			};
		};
		let i64 = self.context.i64_type();
		let start = self.expr(start)?;
		let start = self.convert(start, Repr::Int(IntRepr::I64), iterable.index())?;
		let (end, inclusive) = match end {
			Some(end) => {
				let end = self.expr(end)?;
				let end = self.convert(end, Repr::Int(IntRepr::I64), iterable.index())?;
				(end.int(), inclusive)
			}
			None => (i64.const_int(i64::MAX as u64, true), true),
		};
		let counter =
			match pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern)) {
				Some(symbol) => self.slot_of(symbol, node)?,
				None => self.alloca(Repr::Int(IntRepr::I64), "counter")?,
			};
		self.builder.build_store(counter.pointer, start.int())?;
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
		self.frame_mut().loops.push(LoopFrame {
			node,
			exit,
			next: step,
			value: None,
		});
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
		self.frame_mut().loops.push(LoopFrame {
			node,
			exit,
			next: body_block,
			value,
		});
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
			Some(slot) => self.load(slot),
			None => Ok(Val::void()),
		}
	}

	fn break_statement(&mut self, node: NodeId, value: Option<NodeId>) -> Result<Val<'ctx>, Error> {
		let Some((exit, slot, _)) = self
			.frame()
			.loops
			.last()
			.map(|frame| (frame.exit, frame.value, frame.node))
		else {
			return Err(self.unsupported(node.index(), "`break` outside a loop"));
		};
		if let Some(value) = value {
			let lowered = self.expr(value)?;
			if lowered.repr == Repr::Never {
				return Ok(Val::never());
			}
			if let Some(slot) = slot {
				self.store(slot, lowered, value)?;
			}
		}
		self.builder.build_unconditional_branch(exit)?;
		self.continue_unreachable();
		Ok(Val::never())
	}

	/// `assert condition`: a stop with the source of the condition when false.
	fn assert(&mut self, node: NodeId, condition: NodeId) -> Result<Val<'ctx>, Error> {
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
