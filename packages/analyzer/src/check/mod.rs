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

pub(crate) mod arrays;
mod calls;
mod control;
mod decl;
mod errors;
mod expr;
mod functions;
mod generics;
mod narrowing;
mod numbers;
mod objects;
mod ops;
mod optionals;
mod patterns;
mod proofs;
mod strings;
mod tests;
mod type_decls;
mod type_expr;

#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod function_tests;
#[cfg(test)]
mod generic_tests;
#[cfg(test)]
mod type_tests;
#[cfg(test)]
mod walk_tests;

use std::sync::Arc;

use rustc_hash::FxHashMap;

use fa_parser::TypedSyntaxTree;

use crate::analysis::{Analysis, NodeFlags};
use crate::diagnostics::{self, Code, Diagnostic, Location, SourceSpan, codes};
use crate::flow::facts::FactStore;
use crate::flow::{FlowState, Places};
use crate::ids::{FileId, NodeId, PlaceId, ScopeId, SymbolId, TypeId};
use crate::session::Session;
use crate::symbols::{Resolution, ScopeKind};
use crate::syntax::{self, Name, View};
use crate::types::{Mode, Reason, TypeKind, TypeStore};

/// The function being analyzed (design §3.5). Grows with the function work.
#[derive(Clone, Debug)]
pub struct FnCx {
	/// The lambda node.
	pub node: NodeId,
	/// The function's symbol (`None` for a lambda), for messages.
	pub symbol: Option<SymbolId>,
	/// What `return value` must give: the declared return type (or the one of
	/// the expected function type of a lambda); `VOID` for a block body without
	/// return type; `UNSET` when inferred from the body.
	pub returns: TypeId,
	/// With `returns` `UNSET`: whether `return value` is allowed and joins the
	/// inferred return type (lambdas). A `function` without return type
	/// rejects it (`FN013`).
	pub infer_returns: bool,
	/// The join of the values given to `return` when inferred (`NEVER`: none yet).
	pub returned: TypeId,
	/// The accumulated `throws` (`NEVER`: none).
	pub throws: TypeId,
	/// The flow states at `return`.
	pub return_states: Vec<FlowState>,
	/// [`Walker::unsupported_seen`] when the body started: diagnostics that
	/// depend on the whole body (a missing final value) are dropped when an
	/// unsupported construct was met since.
	pub unsupported_at_start: u32,
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
	/// `loop` (which has a value), or `for`/`while` (which don't).
	pub kind: control::LoopKind,
	/// The expected type of a `loop` used as a value; `None` for a statement.
	pub expected: Option<TypeId>,
	/// Whether the `loop`'s value is used.
	pub value_used: bool,
	/// `functions.len()` when the loop started: a deeper `break` crosses a function.
	pub function_depth: usize,
	/// The types of the `break` values of the current round.
	pub break_types: Vec<(NodeId, TypeId)>,
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
	/// The open `try` and `catch` operands, innermost last (`errors.rs`).
	pub handlers: Vec<errors::Handler>,
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
			handlers: Vec::new(),
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
	/// so that an error is not reported twice. For a field or an element:
	/// whether its root may be changed.
	pub assignable: bool,
	/// A field or an element below the root (`a.b`, `a[0]`): a non-assignable
	/// root is `E-DECL-4` instead of `E-DECL-3`.
	pub below: bool,
}

impl PlaceInfo {
	/// A place the walk could not resolve (already reported).
	pub fn unknown() -> Self {
		Self {
			symbol: None,
			place: None,
			ty: TypeId::UNKNOWN,
			assignable: true,
			below: false,
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
	pub(crate) quiet: u32,
	/// The number of `AN001` met so far, reported or not, and of statements
	/// with a parse error: what follows them may be incomplete.
	pub(crate) unsupported_seen: u32,
	/// The scope of the functions of each `type` with a body, and the type
	/// (`self`, `Self`).
	pub(crate) type_scopes: FxHashMap<ScopeId, (TypeId, NodeId)>,
	/// The `type` declarations of each declared type, for its methods and
	/// associated functions (a structural type found through its spelling).
	pub(crate) decls_by_type: FxHashMap<TypeId, Vec<NodeId>>,
	/// How many nominal types of each name were declared (`DeclKey` disambiguator).
	pub(crate) nominal_names: FxHashMap<String, u16>,
	/// The generic parameters of each generic declaration node.
	pub(crate) generic_params: FxHashMap<NodeId, Vec<crate::ids::ParamId>>,
	/// The parameters of the generic declarations being walked: rigid, not inferred.
	pub(crate) rigid_params: Vec<crate::ids::ParamId>,
	/// The bindings of each instance of a generic type (its functions are substituted).
	pub(crate) instance_bindings: FxHashMap<TypeId, crate::types::Bindings>,
	/// The type parameters of the generic built-in methods (`Array.map`), per owner.
	pub(crate) builtin_params: FxHashMap<&'static str, TypeId>,
	/// Inside the constraint of a generic parameter (a union there is not a runtime union).
	pub(crate) constraint_depth: u32,
	/// The type diagnostics reported on nodes checked more than once.
	pub(crate) reported_once: rustc_hash::FxHashSet<(NodeId, &'static str)>,
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
		unsupported_seen: 0,
		type_scopes: FxHashMap::default(),
		decls_by_type: FxHashMap::default(),
		nominal_names: FxHashMap::default(),
		reported_once: rustc_hash::FxHashSet::default(),
		generic_params: FxHashMap::default(),
		rigid_params: Vec::new(),
		constraint_depth: 0,
		builtin_params: FxHashMap::default(),
		instance_bindings: FxHashMap::default(),
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
		if ty == TypeId::NEVER {
			// the code after a `Never` expression is unreachable
			self.body.flow.set_unreachable();
		}
		ty
	}

	/// Checks a node in statement position: an `if` or a `when` there is a
	/// statement (`Void`), not a value. Handlers ask [`Walker::is_statement`].
	/// A statement with a parse error is walked quietly: the parse error stands
	/// for whatever else is wrong in it.
	pub fn check_stmt(&mut self, n: NodeId) {
		let broken = self.is_broken(n);
		self.unsupported_seen += u32::from(broken);
		let saved = self.statement.replace(n);
		self.quiet += u32::from(broken);
		self.check(n, None);
		self.quiet -= u32::from(broken);
		self.statement = saved;
	}

	/// Checks the last statement of a block whose value is used (see
	/// [`Walker::check_block_value`]): a value, walked quietly like a
	/// statement when it touches a parse error.
	pub fn check_value_stmt(&mut self, n: NodeId, expected: Option<TypeId>) -> TypeId {
		let broken = self.is_broken(n);
		self.unsupported_seen += u32::from(broken);
		self.quiet += u32::from(broken);
		let ty = self.check(n, expected);
		self.quiet -= u32::from(broken);
		ty
	}

	/// Whether a statement touches a parse error.
	fn is_broken(&self, n: NodeId) -> bool {
		let span = self.span(n);
		!matches!(self.view(n), View::Module { .. } | View::Block { .. })
			&& self
				.parse_errors
				.iter()
				.any(|error| error.start <= span.end && span.start <= error.end)
	}

	/// Whether `n` is being checked in statement position.
	pub fn is_statement(&self, n: NodeId) -> bool {
		self.statement == Some(n)
	}

	/// Checks a condition and returns the flow states where it is true and
	/// where it is false (design §3.7, `ops.rs`).
	pub fn check_cond(&mut self, n: NodeId) -> (FlowState, FlowState) {
		self.condition(n, true)
	}

	/// Checks an assignment target, a `use` source or an effect argument: a
	/// name, a field (`objects.rs`), an element.
	pub fn check_place(&mut self, n: NodeId) -> PlaceInfo {
		let tree = Arc::clone(&self.tree);
		let info = match syntax::view(&tree, n) {
			View::Name(name) => {
				let ty = self.check_name(n, name);
				let symbol = match self.a.resolution(n) {
					Resolution::Local(symbol) => Some(symbol),
					_ => None,
				};
				// a place has its declared type (the read may be narrowed: narrowing.rs)
				let ty = symbol.map_or(ty, |symbol| {
					let declared = self.a.symbols.symbol(symbol).ty;
					if declared.is_unset() { ty } else { declared }
				});
				let assignable = match self.a.resolution(n) {
					Resolution::Local(symbol) => {
						let symbol = self.a.symbols.symbol(symbol);
						// a symbol of an unsupported declaration: already reported
						symbol.kind.is_assignable() || self.s.types.is_recovering(symbol.ty)
					}
					Resolution::Unresolved => true,
					_ => false,
				};
				PlaceInfo {
					symbol,
					place: None,
					ty,
					assignable,
					below: false,
				}
			}
			View::Access { target, members } => self.check_field_place(n, target, members),
			View::Index { target, index } => self.check_index_place(n, target, index),
			View::Group(inner) => self.check_place(inner),
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
		if self.is_integer_sink(actual, target) {
			return self.sink_integer(n, actual, target);
		}
		// a generic function given where a function type is expected (generics.rs)
		let actual = match self.instantiate_function_value(n, actual, target) {
			Some(instance) if self.s.types.is_recovering(instance) => return,
			Some(instance) => instance,
			None => actual,
		};
		match self.s.types.assignable(actual, target, Mode::Copy) {
			Ok(()) => {
				// a `Character` stored as a `String` is converted
				if self.s.types.kind(actual) == TypeKind::Character
					&& self.s.types.kind(target) == TypeKind::String
				{
					self.a.coercions.insert(n, target);
				}
				self.sink_union_member(n, actual, target);
			}
			Err(Reason::Range) => self.sink_integer(n, actual, target),
			Err(_) if self.string_sink_mismatch(n, actual, target) => {}
			Err(_) if actual == TypeId::NONE => {
				let message = format!(
					"a `{}` can't be `none`: make the type optional, ex: `{}?`",
					self.display(target),
					self.display(target)
				);
				let diagnostic = self.diagnostic(&codes::values::E_OPT_2, n, message);
				self.report_flow(diagnostic);
			}
			Err(Reason::MissingField)
				if matches!(
					self.s.types.kind(actual),
					TypeKind::Object(_) | TypeKind::Member(..)
				) =>
			{
				// an existing value must have every field, even an optional one
				let missing = self
					.s
					.types
					.fields_of(target)
					.iter()
					.find(|field| {
						!self
							.s
							.types
							.fields_of(actual)
							.iter()
							.any(|other| other.name == field.name)
					})
					.map(|field| self.s.types.syms.resolve(field.name).to_string())
					.unwrap_or_default();
				let message = format!(
					"a `{}` has no `{missing}` field: a `{}` is expected (only a literal may leave an optional field out)",
					self.display(actual),
					self.display(target)
				);
				let diagnostic = self.diagnostic(&codes::types::TY026, n, message);
				self.report_flow(diagnostic);
			}
			Err(_) if self.optional_sink(n, actual, target) => {}
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

	/// A value stored in an anonymous union: `TY041` when it satisfies several
	/// object members (it couldn't be told apart later), and the member it is
	/// converted to (`coercions`).
	fn sink_union_member(&mut self, n: NodeId, actual: TypeId, target: TypeId) {
		let TypeKind::Union(list) = self.s.types.kind(target) else {
			return;
		};
		if self.s.types.kind(actual) == TypeKind::Union(list)
			|| !matches!(self.s.types.kind(actual), TypeKind::Object(_))
		{
			return;
		}
		let members = self.s.types.list_items(list).to_vec();
		if members.contains(&actual) {
			return;
		}
		let fitting: Vec<TypeId> = members
			.into_iter()
			.filter(|&member| {
				matches!(self.s.types.kind(member), TypeKind::Object(_))
					&& self.s.types.assignable(actual, member, Mode::Copy).is_ok()
			})
			.collect();
		match fitting.as_slice() {
			[member] => {
				self.a.coercions.insert(n, *member);
			}
			[first, second, ..] => {
				let message = format!(
					"a `{}` is both a `{}` and a `{}`: copy it into one of them first",
					self.display(actual),
					self.display(*first),
					self.display(*second)
				);
				let diagnostic = self.diagnostic(&codes::types::TY041, n, message);
				self.report_flow(diagnostic);
			}
			[] => {}
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
		self.count_unsupported(&diagnostic);
		if self.quiet == 0 {
			self.output.push(diagnostic);
		}
	}

	/// Reports a flow diagnostic: into the innermost flow buffer, or the output
	/// when there is none (decision A3).
	pub fn report_flow(&mut self, diagnostic: Diagnostic) {
		self.count_unsupported(&diagnostic);
		if self.quiet > 0 {
			return;
		}
		match self.body.flow_buffers.last_mut() {
			Some(buffer) => buffer.push(diagnostic),
			None => self.output.push(diagnostic),
		}
	}

	fn count_unsupported(&mut self, diagnostic: &Diagnostic) {
		if std::ptr::eq(diagnostic.code, &codes::analyzer::AN001) {
			self.unsupported_seen += 1;
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

	/// The view of a node. It borrows the walker: to call handlers while
	/// holding a view, use `syntax::view(&Arc::clone(&self.tree), n)`.
	pub fn view(&self, n: NodeId) -> View<'_> {
		syntax::view(&self.tree, n)
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
