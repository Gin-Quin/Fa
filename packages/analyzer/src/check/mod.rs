//! The walk (design §3): one traversal of a file's tree that resolves names,
//! computes types, runs the flow analysis and emits diagnostics.
//!
//! [`Walker::check`] matches exhaustively on [`View`] and calls one handler per
//! variant; the handlers live in the files of this directory, one area each
//! (design §5), so that engineers fill them without touching this dispatch.
//! Every handler follows the same pattern (decision A1, §8 of the design):
//!
//! ```text
//! children: self.check(child, expected_for_child)…   every visit: they carry flow
//! ty = self.memo(n, |w| type rule + type diagnostics (w.report))
//! flow transfer of n: facts, check bits, sinks, w.report_flow(…)
//! ```
//!
//! The type part runs once per node (`types[n]` is set); the flow part runs at
//! every visit (loop rounds, SCC rounds), and its diagnostics go through the
//! flow buffers (decision A3) so that only the stable round's are kept.

mod calls;
mod control;
mod decl;
mod errors;
mod expr;
mod functions;
mod numbers;
mod objects;
mod ops;
mod patterns;
mod strings;
mod tests;
mod type_expr;

#[cfg(test)]
mod walk_tests;

use std::sync::Arc;

use fa_parser::TypedSyntaxTree;

use crate::analysis::{Analysis, NodeFlags};
use crate::diagnostics::{self, Code, Diagnostic, Location, SourceSpan, codes};
use crate::flow::facts::FactStore;
use crate::flow::{FlowState, Places};
use crate::ids::{FileId, NodeId, PlaceId, ScopeId, SymbolId, TypeId};
use crate::session::Session;
use crate::symbols::{Resolution, ScopeKind};
use crate::syntax::{self, Name, View};
use crate::types::{Mode, Reason, TypeStore};

/// The function being analyzed (design §3.5). Grows with the function work.
#[derive(Clone, Debug)]
pub struct FnCx {
	/// The function or lambda node.
	pub node: NodeId,
	/// The declared return type, `UNSET` when inferred.
	pub returns: TypeId,
	/// The accumulated `throws` (`NEVER`: none).
	pub throws: TypeId,
	/// The flow states at `return`.
	pub return_states: Vec<FlowState>,
}

/// The loop being analyzed (design §3.3).
#[derive(Clone, Debug)]
pub struct LoopCx {
	/// The loop node.
	pub node: NodeId,
	/// The flow states at `break`.
	pub breaks: Vec<FlowState>,
	/// The flow states at `continue`.
	pub continues: Vec<FlowState>,
	/// The value type of a `loop` (`break value`), `UNSET` until a `break` gives one.
	pub value: TypeId,
}

/// A typing that depended on narrowing, re-validated at every visit (decision A2).
#[derive(Copy, Clone, Debug)]
pub struct Requirement {
	/// The node whose type relied on the narrowing.
	pub node: NodeId,
	/// The narrowed place.
	pub place: PlaceId,
	/// The type the place must still have.
	pub needs: TypeId,
}

/// The context of the body being walked, swapped wholesale for on-demand
/// analysis of a hoisted declaration (design §3.2).
#[derive(Clone, Debug)]
pub struct BodyCx {
	/// The current scope.
	pub scope: ScopeId,
	/// The current flow state.
	pub flow: FlowState,
	/// The enclosing functions, innermost last.
	pub functions: Vec<FnCx>,
	/// The enclosing loops, innermost last.
	pub loops: Vec<LoopCx>,
	/// Flow diagnostics of the current loop iteration / SCC round (decision A3).
	pub flow_buffers: Vec<Vec<Diagnostic>>,
	/// Narrowing requirements to re-validate (decision A2).
	pub requirements: Vec<Requirement>,
}

impl BodyCx {
	/// A fresh body in `scope`: empty flow state, no buffer (decision A11).
	pub fn new(scope: ScopeId) -> Self {
		Self {
			scope,
			flow: FlowState::new(),
			functions: Vec::new(),
			loops: Vec::new(),
			flow_buffers: Vec::new(),
			requirements: Vec::new(),
		}
	}
}

/// An assignment target, `use` source or effect argument.
#[derive(Copy, Clone, Debug)]
pub struct PlaceInfo {
	/// The root symbol, when it is a local.
	pub symbol: Option<SymbolId>,
	/// The flow place (M1b: fields, indices).
	pub place: Option<PlaceId>,
	/// The declared type of the place.
	pub ty: TypeId,
	/// Whether it may be assigned as a whole (`mutable`). True when unknown,
	/// so that an error is not reported twice.
	pub assignable: bool,
}

impl PlaceInfo {
	/// A place the walk could not resolve (already reported).
	pub fn unknown() -> Self {
		Self {
			symbol: None,
			place: None,
			ty: TypeId::UNKNOWN,
			assignable: true,
		}
	}
}

/// The walker of one file.
pub struct Walker<'s> {
	/// The session: the type store, other files (M2).
	pub s: &'s mut Session,
	/// The file being walked.
	pub file: FileId,
	/// Its tree.
	pub tree: Arc<TypedSyntaxTree>,
	/// Its side tables, taken out of the session during the walk.
	pub a: Analysis,
	/// The body being walked.
	pub body: BodyCx,
	/// The places of the file's flow states.
	pub places: Places,
	/// The facts of the file's flow states.
	pub facts: FactStore,
	/// The declarations being analyzed, innermost last (`DeclState` frames).
	pub decl_stack: Vec<SymbolId>,
	/// The node checked by [`Walker::check_stmt`], while its handler runs.
	statement: Option<NodeId>,
	/// Inside an unsupported construct: diagnostics are dropped (its `AN001` stands for them).
	quiet: u32,
	/// The analysis' own diagnostics, sorted at the end.
	output: Vec<Diagnostic>,
	/// The spans of the parse errors: a statement touching one is walked quietly.
	parse_errors: Vec<SourceSpan>,
}

/// Runs the walk over a file and stores its analysis in the session.
pub fn analyze(session: &mut Session, file: FileId) {
	let (tree, mut analysis) = session.take_analysis(file);
	let root = NodeId::new(tree.root);
	let scope = analysis.symbols.scope_for(root, None, ScopeKind::Module);
	let parse_errors = tree
		.diagnostics
		.iter()
		.filter(|diagnostic| diagnostic.severity == fa_parser::Severity::Error)
		.map(|diagnostic| SourceSpan::new(diagnostic.span.start as u32, diagnostic.span.end as u32))
		.collect();
	let mut walker = Walker {
		s: session,
		file,
		tree,
		a: analysis,
		body: BodyCx::new(scope),
		places: Places::new(),
		facts: FactStore::new(),
		decl_stack: Vec::new(),
		statement: None,
		quiet: 0,
		output: Vec::new(),
		parse_errors,
	};
	if root.index() < walker.tree.nodes.len() {
		walker.check_stmt(root);
	}
	let Walker {
		s, mut a, output, ..
	} = walker;
	let mut output = output;
	diagnostics::sort(&mut output);
	a.diagnostics.extend(output);
	s.put_analysis(file, a);
}

impl Walker<'_> {
	/// Checks an expression in value position and returns its type. `expected`
	/// is the type the context expects (a sink, a parameter), used for literal
	/// adaptation and contextual typing; it is not a check (see [`Walker::sink`]).
	pub fn check(&mut self, n: NodeId, expected: Option<TypeId>) -> TypeId {
		let tree = Arc::clone(&self.tree);
		let ty = match syntax::view(&tree, n) {
			// structure (decl.rs)
			View::Module { statements } => self.check_module(n, statements),
			View::Block { statements } => self.check_block(n, statements, expected),
			View::Error => TypeId::UNKNOWN,
			View::Unsupported(kind) => self.unsupported(n, kind),

			// literals, names, values (expr.rs)
			View::Integer(value) => self.check_integer(n, value, expected),
			View::Boolean(value) => self.check_boolean(n, value),
			View::None => self.check_none(n),
			View::String(text) => self.check_string(n, text, expected),
			View::Template { parts } => self.check_template(n, parts),
			View::Name(name) => self.check_name(n, name),
			View::ImplicitMember(name) => self.check_implicit_member(n, name, expected),
			View::Group(inner) => self.check_group(n, inner, expected),
			View::EmptyGroup => self.check_empty_group(n),
			View::Array { items } => self.check_array(n, items, expected),
			View::Tuple { items } => self.check_tuple(n, items, expected),
			View::Spread(value) => self.check_spread(n, value),
			View::Range {
				start,
				end,
				inclusive,
			} => self.check_range(n, start, end, inclusive),
			View::Access { target, members } => self.check_access(n, target, members),
			View::Index { target, index } => self.check_index(n, target, index),

			// numbers (numbers.rs)
			View::Float(value) => self.check_float(n, value, expected),
			View::BigInt(text) => self.check_big_int(n, text),
			View::Arithmetic { op, operands } => self.check_arithmetic(n, op, operands, expected),
			View::Negate(value) => self.check_negate(n, value, expected),
			View::Percentage(value) => self.check_percentage(n, value),
			View::CompoundAssignment { op, target, value } => {
				self.check_compound_assignment(n, op, target, value)
			}

			// comparisons, logic, optionals (ops.rs)
			View::Equality {
				left,
				right,
				negated,
			} => self.check_equality(n, left, right, negated),
			View::Comparison {
				operands,
				operators,
			} => self.check_comparison(n, operands, operators),
			View::Logic { op, operands } => self.check_logic(n, op, operands),
			View::Not(value) => self.check_not(n, value),
			View::Fallback { operands } => self.check_fallback(n, operands, expected),
			View::Is { value, ty, negated } => self.check_is(n, value, ty, negated),
			View::IsIn {
				value,
				collection,
				negated,
			} => self.check_is_in(n, value, collection, negated),
			View::OptionalAccess { target, members } => {
				self.check_optional_access(n, target, members)
			}

			// declarations (decl.rs)
			View::Declaration {
				kind,
				name,
				annotation,
				value,
			} => self.check_declaration(n, kind, name, annotation, value),
			View::Use {
				name,
				annotation,
				value,
				mutable,
			} => self.check_use(n, name, annotation, value, mutable),
			View::Reuse { name, value } => self.check_reuse(n, name, value),
			View::Assignment {
				target,
				annotation,
				value,
			} => self.check_assignment(n, target, annotation, value),
			View::TypeDecl {
				name,
				parameters,
				value,
			} => self.hoisted(n, |w| w.check_type_decl(n, name, parameters, value)),
			View::Namespace { name, value } => {
				self.hoisted(n, |w| w.check_namespace(n, name, value))
			}
			View::ExportValue {
				mutable,
				annotation,
				value,
			} => self.check_export_value(n, mutable, annotation, value),
			View::ExportFunction {
				is_async,
				generics,
				annotation,
				value,
			} => self.check_export_function(n, is_async, generics, annotation, value),
			View::ExportType { parameters, value } => self.check_export_type(n, parameters, value),
			View::ExportNamespace(value) => self.check_export_namespace(n, value),

			// functions (functions.rs) and calls (calls.rs)
			View::Function {
				name,
				is_async,
				generics,
				annotation,
				value,
			} => self.hoisted(n, |w| {
				w.check_function_decl(n, name, is_async, generics, annotation, value)
			}),
			View::Lambda { signature, body } => self.check_lambda(n, signature, body, expected),
			View::Parameters { items } => self.check_parameters(n, items),
			View::DestructuredParameter {
				pattern,
				annotation,
			} => self.check_destructured_parameter(n, pattern, annotation),
			View::Compose { operands } => self.check_compose(n, operands),
			View::PartialApplication {
				function,
				arguments,
			} => self.check_partial_application(n, function, arguments),
			View::Provide { ty, value } => self.check_provide(n, ty, value),
			View::Call { callee, arguments } => self.check_call(n, callee, arguments, expected),

			// control flow (control.rs)
			View::If {
				condition,
				then_body,
				else_body,
				compile_time,
			} => self.check_if(n, condition, then_body, else_body, compile_time, expected),
			View::When { subject, arms } => self.check_when(n, subject, arms, expected),
			View::While { condition, body } => self.check_while(n, condition, body),
			View::WhileLet {
				pattern,
				value,
				body,
			} => self.check_while_let(n, pattern, value, body),
			View::For {
				pattern,
				index,
				iterable,
				body,
				parallel,
				compile_time,
			} => self.check_for(n, pattern, index, iterable, body, parallel, compile_time),
			View::Loop(body) => self.check_loop(n, body, expected),
			View::Do(body) => self.check_do(n, body, expected),
			View::Break(value) => self.check_break(n, value),
			View::Continue => self.check_continue(n),
			View::Return(value) => self.check_return(n, value),

			// errors (errors.rs)
			View::Try(value) => self.check_try(n, value, expected),
			View::Throw(value) => self.check_throw(n, value),
			View::Catch {
				expression,
				error_name,
				handler,
			} => self.check_catch(n, expression, error_name, handler, expected),

			// tests (tests.rs)
			View::Test {
				name,
				modifier,
				body,
				compile_time,
			} => self.check_test(n, name, modifier, body, compile_time),
			View::Assert(condition) => self.check_assert(n, condition),

			// objects (objects.rs)
			View::Object { items } => self.check_object(n, items, expected),
			View::TypedObject { target, members } => self.check_typed_object(n, target, members),
			View::UpdateFields { target, value } => self.check_update_fields(n, target, value),
			View::Readonly(field) => self.check_readonly(n, field),

			// patterns (patterns.rs)
			View::Destructuring {
				kind,
				pattern,
				value,
			} => self.check_destructuring(n, kind, pattern, value),
			View::ObjectPattern { fields, rest } => self.check_object_pattern(n, fields, rest),
			View::ArrayPattern { items, rest } => self.check_array_pattern(n, items, rest),
			View::TuplePattern { items } => self.check_tuple_pattern(n, items),

			// type-only constructs in value position (type_expr.rs)
			View::ErrorType
			| View::Union { .. }
			| View::Optional(_)
			| View::RefType { .. }
			| View::Throws { .. }
			| View::Requires { .. }
			| View::Effect { .. } => self.check_type_in_value_position(n),
		};
		if let Some(slot) = self.a.types.get_mut(n.index())
			&& slot.is_unset()
		{
			*slot = ty;
		}
		ty
	}

	/// Checks a node in statement position: an `if` or a `when` there is a
	/// statement (`Void`), not a value. Handlers ask [`Walker::is_statement`].
	/// A statement with a parse error is walked quietly: the parse error stands
	/// for whatever else is wrong in it.
	pub fn check_stmt(&mut self, n: NodeId) {
		let span = self.span(n);
		let broken = !matches!(
			syntax::view(&Arc::clone(&self.tree), n),
			View::Module { .. } | View::Block { .. }
		) && self
			.parse_errors
			.iter()
			.any(|error| error.start <= span.end && span.start <= error.end);
		let saved = self.statement.replace(n);
		self.quiet += u32::from(broken);
		self.check(n, None);
		self.quiet -= u32::from(broken);
		self.statement = saved;
	}

	/// Whether `n` is being checked in statement position.
	pub fn is_statement(&self, n: NodeId) -> bool {
		self.statement == Some(n)
	}

	/// Checks a condition and returns the flow states where it is true and
	/// where it is false (design §3.7). Stub: no narrowing yet (flow/narrowing.rs).
	pub fn check_cond(&mut self, n: NodeId) -> (FlowState, FlowState) {
		let ty = self.check(n, Some(TypeId::BOOLEAN));
		self.sink(n, ty, TypeId::BOOLEAN);
		(self.body.flow.fork(), self.body.flow.fork())
	}

	/// Checks an assignment target, a `use` source or an effect argument.
	/// Names only for now; fields and indices come with objects and arrays.
	pub fn check_place(&mut self, n: NodeId) -> PlaceInfo {
		let tree = Arc::clone(&self.tree);
		let info = match syntax::view(&tree, n) {
			View::Name(name) => {
				let ty = self.check_name(n, name);
				let symbol = match self.a.resolution(n) {
					Resolution::Local(symbol) => Some(symbol),
					_ => None,
				};
				let assignable = match self.a.resolution(n) {
					Resolution::Local(symbol) => self.a.symbols.symbol(symbol).kind.is_assignable(),
					Resolution::Unresolved => true,
					_ => false,
				};
				PlaceInfo {
					symbol,
					place: None,
					ty,
					assignable,
				}
			}
			View::Error => PlaceInfo::unknown(),
			_ => {
				self.unsupported(n, "this assignment target");
				PlaceInfo::unknown()
			}
		};
		self.set_flag(n, NodeFlags::PLACE);
		info
	}

	/// A value of type `actual` (node `n`) flows into a place of type `target`
	/// (values brief §2.3). A flow-part operation: call it at every visit; its
	/// diagnostics go through [`Walker::report_flow`]. Integer ranges are
	/// delegated to `numbers.rs` ([`Walker::sink_integer`]).
	pub fn sink(&mut self, n: NodeId, actual: TypeId, target: TypeId) {
		if actual.is_unset()
			|| target.is_unset()
			|| self.s.types.is_recovering(actual)
			|| self.s.types.is_recovering(target)
		{
			return;
		}
		match self.s.types.assignable(actual, target, Mode::Copy) {
			Ok(()) => {}
			Err(Reason::Range) => self.sink_integer(n, actual, target),
			Err(_) => {
				let message = format!(
					"a `{}` is not accepted where a `{}` is expected",
					self.display(actual),
					self.display(target)
				);
				let diagnostic = self.diagnostic(&codes::values::E_TYPE_1, n, message);
				self.report_flow(diagnostic);
			}
		}
	}

	/// The type part of a node: runs `f` only if the node has no type yet, and
	/// stores its result. `f` must return a set type (`VOID` for a statement).
	pub fn memo(&mut self, n: NodeId, f: impl FnOnce(&mut Self) -> TypeId) -> TypeId {
		let current = self.a.type_of(n);
		if !current.is_unset() {
			return current;
		}
		let ty = f(self);
		self.set_type(n, ty);
		ty
	}

	/// Reports a type diagnostic (from inside [`Walker::memo`]): emitted once.
	pub fn report(&mut self, diagnostic: Diagnostic) {
		if self.quiet == 0 {
			self.output.push(diagnostic);
		}
	}

	/// Reports a flow diagnostic: into the innermost flow buffer, or the output
	/// when there is none (decision A3).
	pub fn report_flow(&mut self, diagnostic: Diagnostic) {
		if self.quiet > 0 {
			return;
		}
		match self.body.flow_buffers.last_mut() {
			Some(buffer) => buffer.push(diagnostic),
			None => self.output.push(diagnostic),
		}
	}

	/// Starts buffering the flow diagnostics of a loop round or an SCC round.
	pub fn push_flow_buffer(&mut self) {
		self.body.flow_buffers.push(Vec::new());
	}

	/// Ends the current round: `keep` its diagnostics (the stable round: they go
	/// to the enclosing buffer, or the output) or discard them.
	pub fn pop_flow_buffer(&mut self, keep: bool) {
		let Some(buffer) = self.body.flow_buffers.pop() else {
			return;
		};
		if keep {
			for diagnostic in buffer {
				self.report_flow(diagnostic);
			}
		}
	}

	/// A diagnostic on node `n`.
	pub fn diagnostic(&self, code: Code, n: NodeId, message: impl Into<String>) -> Diagnostic {
		self.diagnostic_at(code, self.span(n), message)
	}

	/// A diagnostic on a span of the file.
	pub fn diagnostic_at(
		&self,
		code: Code,
		span: SourceSpan,
		message: impl Into<String>,
	) -> Diagnostic {
		Diagnostic::new(
			code,
			Location {
				file: self.file,
				span,
			},
			message,
		)
	}

	/// The `AN001` stub of a construct not handled yet: reported once, type
	/// `UNKNOWN`; its children are still walked (names resolve for the IDE),
	/// with their diagnostics dropped.
	pub fn unsupported(&mut self, n: NodeId, what: &str) -> TypeId {
		self.memo(n, |w| {
			let message = format!("{what} is not supported yet by the analyzer");
			let diagnostic = w.diagnostic(&codes::analyzer::AN001, n, message);
			w.report(diagnostic);
			TypeId::UNKNOWN
		});
		self.quiet += 1;
		self.walk_children(n);
		self.quiet -= 1;
		TypeId::UNKNOWN
	}

	/// Checks every child of `n` in value position, with no expected type.
	pub fn walk_children(&mut self, n: NodeId) {
		for child in syntax::children(&self.tree, n) {
			self.check(child, None);
		}
	}

	/// Runs the analysis of a hoisted declaration (`function`, `type`,
	/// `namespace`) once: reached in order or on demand, whichever comes first
	/// (design §3.2). `f` sets the symbol's type; `UNKNOWN` if it doesn't.
	fn hoisted(&mut self, n: NodeId, f: impl FnOnce(&mut Self) -> TypeId) -> TypeId {
		use crate::symbols::DeclState;
		let Some(symbol) = self.a.symbols.symbol_of_decl(n) else {
			return f(self);
		};
		match self.a.symbols.symbol(symbol).state {
			DeclState::Pending | DeclState::Sequential => {}
			_ => return TypeId::VOID,
		}
		let frame = self.decl_stack.len() as u32;
		self.a.symbols.symbol_mut(symbol).state = DeclState::Signature { frame };
		self.decl_stack.push(symbol);
		let ty = f(self);
		self.decl_stack.pop();
		let symbol = self.a.symbols.symbol_mut(symbol);
		symbol.state = DeclState::Done;
		if symbol.ty.is_unset() {
			symbol.ty = TypeId::UNKNOWN;
		}
		ty
	}

	// ── small accessors ──────────────────────────────────────────────────

	/// The type store.
	pub fn types(&mut self) -> &mut TypeStore {
		&mut self.s.types
	}

	/// Prints a type for a message.
	pub fn display(&self, ty: TypeId) -> String {
		self.s.types.display(ty)
	}

	/// The span of a node.
	pub fn span(&self, n: NodeId) -> SourceSpan {
		syntax::span(&self.tree, n)
	}

	/// The text of a name.
	pub fn name_text(&self, name: Name) -> &str {
		self.tree.name(name)
	}

	/// The type recorded for a node.
	pub fn type_of(&self, n: NodeId) -> TypeId {
		self.a.type_of(n)
	}

	/// Records the type of a node.
	pub fn set_type(&mut self, n: NodeId, ty: TypeId) {
		if let Some(slot) = self.a.types.get_mut(n.index()) {
			*slot = ty;
		}
	}

	/// Records what a name or member refers to.
	pub fn set_resolution(&mut self, n: NodeId, resolution: Resolution) {
		if let Some(slot) = self.a.resolutions.get_mut(n.index()) {
			*slot = resolution;
		}
	}

	/// Sets flags on a node.
	pub fn set_flag(&mut self, n: NodeId, flags: NodeFlags) {
		if let Some(slot) = self.a.flags.get_mut(n.index()) {
			*slot |= flags;
		}
	}

	/// Clears flags of a node (check bits are recomputed at every visit).
	pub fn clear_flag(&mut self, n: NodeId, flags: NodeFlags) {
		if let Some(slot) = self.a.flags.get_mut(n.index()) {
			slot.remove(flags);
		}
	}
}
