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
mod c_library;
mod callable_values;
mod callbacks;
mod calls;
mod collections;
mod compose;
mod comptime;
mod concurrency;
mod control;
mod decl;
mod destructuring;
pub mod effects;
mod errors;
mod expr;
mod fields;
mod foreign;
mod from;
mod functions;
mod generics;
mod interface;
mod iterators;
mod key_sets;
mod locals;
mod math;
pub(crate) mod modules;
mod moves;
mod narrowing;
mod numbers;
mod objects;
mod ops;
mod optionals;
mod patterns;
mod pools;
mod proofs;
pub(crate) mod reactivity;
mod references;
mod relations;
mod requirements;
mod retention;
mod schemas;
mod spelling;
mod spreads;
mod std_library;
mod strings;
mod structural;
pub(crate) mod tasks;
mod tests;
mod tuples;
mod type_decls;
mod type_expr;
mod type_functions;
mod type_logic;
mod type_ops;
mod unused;
mod value_params;

pub use tests::is_test_module;

#[cfg(test)]
mod collection_tests;
#[cfg(test)]
mod effect_tests;
#[cfg(test)]
mod error_payload_tests;
#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod function_tests;
#[cfg(test)]
mod generic_tests;
#[cfg(test)]
mod representation_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod type_op_tests;
#[cfg(test)]
mod type_tests;
#[cfg(test)]
mod value_tests;
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
	/// Errors declared by this definition (`None`: inferred). A function
	/// type supplied as a value's constraint does not declare its body.
	pub throws_contract: Option<TypeId>,
	/// The flow states at `return`.
	pub return_states: Vec<FlowState>,
	/// [`Walker::unsupported_seen`] when the body started: diagnostics that
	/// depend on the whole body (a missing final value) are dropped when an
	/// unsupported construct was met since.
	pub unsupported_at_start: u32,
	/// The scope of its parameters (and of a block body's declarations).
	pub scope: ScopeId,
	/// A test body (not a closure: its effects are its own).
	pub test: bool,
	/// The effects met so far on places that are not its locals (`effects.rs`).
	pub effects: Vec<effects::EffectEntry>,
	/// The effects written in its signature (`None`: none written).
	pub declared: Option<crate::ids::EffectSetId>,
	/// A function declaration also has an explicit function-type annotation;
	/// its effects must be exact even when the body writes its own clauses.
	pub extra_effect_contract: Option<crate::ids::EffectSetId>,
	/// The effects its body must stay within (`EFF001`–`EFF004`): a named
	/// function with a block body or written effects; `None`: inferred.
	pub coverage: Option<crate::ids::EffectSetId>,
	/// The first path of its signature (`EFF011`).
	pub signature_node: Option<NodeId>,
	/// An effect of the body was not covered (reported): no `EFF011` then.
	pub uncovered: bool,
	/// The locals of enclosing functions it reads or writes (`FnInfo::captures`).
	pub captures: Vec<SymbolId>,
	/// Its declared requirements, in signature order (`requirements.rs`).
	pub requires: Vec<(crate::types::Requirement, SymbolId)>,
	/// A single-expression named function infers the requirements of its calls.
	pub infers_requires: bool,
	/// The keys it inferred.
	pub inferred_requires: Vec<TypeId>,
	/// The value of a `derived` declaration or the body of a `watch` block
	/// (`reactivity.rs`): it can't throw, and isn't a function of its own.
	pub reactive: Option<reactivity::ReactiveBody>,
	/// An async context: an `async` function or lambda, a `start do` (`tasks.rs`).
	pub is_async: bool,
	/// What its body contributes to its async summary (`tasks.rs`).
	pub async_acc: tasks::Acc,
	/// The owner of the reference it returns (`: T in owner`, references.rs).
	pub ref_owner: Option<PlaceId>,
}

impl FnCx {
	/// The context of a function body.
	pub fn new(node: NodeId, symbol: Option<SymbolId>, returns: TypeId, scope: ScopeId) -> Self {
		Self {
			node,
			symbol,
			returns,
			infer_returns: false,
			returned: TypeId::NEVER,
			throws: TypeId::NEVER,
			throws_contract: None,
			return_states: Vec::new(),
			unsupported_at_start: 0,
			scope,
			test: false,
			effects: Vec::new(),
			declared: None,
			extra_effect_contract: None,
			coverage: None,
			signature_node: None,
			uncovered: false,
			captures: Vec::new(),
			requires: Vec::new(),
			infers_requires: false,
			inferred_requires: Vec::new(),
			reactive: None,
			is_async: false,
			async_acc: tasks::Acc::new(),
			ref_owner: None,
		}
	}
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
	/// The hull of the integer `break` values of the current round (`None`
	/// inside: a value that is not an integer).
	pub break_values: Option<Option<crate::types::Interval>>,
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
	/// The function being walked is written inside a loop of an enclosing
	/// body: a `break` or `continue` here is `E-LOOP-13`, not `E-LOOP-12`.
	pub inside_loop: bool,
	/// The `provide` statements walked, in order (`requirements.rs`).
	pub provides: Vec<requirements::Provided>,
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
			inside_loop: false,
			provides: Vec::new(),
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
	/// Whether the type expression about to be checked is the type of a
	/// parameter, the one place where a generic function type is written
	/// (`GN030` elsewhere; `check_parameter_type`).
	pub(crate) parameter_type: bool,
	/// The `TypeFromSchema` applications that are the value of a generic
	/// type function (or a result of its logic): built per application
	/// (`schemas.rs`).
	pub(crate) schema_logic: rustc_hash::FxHashSet<NodeId>,
	/// Inside such an application at its declaration: the `@` calls are
	/// evaluated per application, not there.
	pub(crate) deferred_comptime: u32,
	/// The declaration applying `TypeFromSchema` and the hash of its schema:
	/// the identity of the named unions, enumerations and recursive types it
	/// builds (`schemas.rs`).
	pub(crate) schema_identity: Option<(String, u64)>,
	/// An error thrown by the compile-time code of a type function's
	/// application is `TF004` (`schemas.rs`).
	pub(crate) in_type_function_code: bool,
	/// The bindings of each instance of a generic type (its functions are substituted).
	pub(crate) instance_bindings: FxHashMap<TypeId, crate::types::Bindings>,
	/// The fields written into values typed by an implicit generic
	/// parameter (`duplicate.name = name` of `mutable duplicate = human` with
	/// `Type<human>`): each path and the written type (structural.rs).
	pub(crate) implicit_writes: FxHashMap<crate::ids::ParamId, Vec<(Vec<crate::ids::Sym>, TypeId)>>,
	/// The calls of functions of this file, by callee text and declaration,
	/// with their arguments: checked against `implicit_writes` once every
	/// body is walked (`EF004`, structural.rs).
	pub(crate) implicit_write_calls: Vec<(String, NodeId, Vec<crate::analysis::Argument>)>,
	/// Key sets and `Fields` types (fields.rs).
	pub(crate) key_sets: key_sets::KeySetState,
	/// The type parameters of the generic built-in methods (`Array.map`), per owner.
	pub(crate) builtin_params: FxHashMap<&'static str, TypeId>,
	/// Inside the constraint of a generic parameter (a union there is not a runtime union).
	pub(crate) constraint_depth: u32,
	/// The type diagnostics reported on nodes checked more than once.
	pub(crate) reported_once: rustc_hash::FxHashSet<(NodeId, &'static str)>,
	/// The `state` literal of a `WorkerPool` (`WorkerPool(T) { state = T { … } }`)
	/// is being checked: a worker type's literal builds the state, not a worker.
	pub(crate) pool_state: Option<NodeId>,
	/// The loop variables that are read-only references: set elements and
	/// map keys (`CL031`).
	pub(crate) readonly_elements: rustc_hash::FxHashSet<SymbolId>,
	/// The recursive reference of each type declaration that named itself
	/// while being built (types brief §3.6).
	pub(crate) rec_references: FxHashMap<SymbolId, TypeId>,
	/// Error payload declarations, validated after recursive targets are complete.
	pub(crate) error_payloads: Vec<(NodeId, TypeId)>,
	/// The sink being checked is a view (an argument of a function), not a
	/// copy: no narrowing copy is recorded (structural.rs).
	pub(crate) view_sink: bool,
	/// The `type` declaration written for each binding and parameter, whose
	/// functions its methods are (structural.rs).
	pub(crate) written_decls: FxHashMap<SymbolId, NodeId>,
	/// The same identity across files (a structural TypeId has no owner).
	pub(crate) written_type_keys: FxHashMap<SymbolId, crate::analysis::WrittenType>,
	pub(crate) written_origin_stack: rustc_hash::FxHashSet<NodeId>,
	/// The declaration written for the receiver of the member being resolved.
	pub(crate) receiver_decl: Option<NodeId>,
	pub(crate) receiver_origin: Option<crate::analysis::WrittenType>,
	/// The requirements each lambda (or `watch`) resolved in the code around
	/// it, with a call needing each and the scope of the `provide` (`None`:
	/// the enclosing function's own): a stored lambda can't (`REQ007`), a
	/// watch can't leave the providing block (`REQ010`, requirements.rs).
	pub(crate) lambda_requirements: FxHashMap<NodeId, Vec<(TypeId, NodeId, Option<ScopeId>)>>,
	/// The parameters of the function being declared that `Type(parameter)`
	/// names, and their hidden generic parameters (structural.rs).
	pub(crate) pending_implicit: Vec<(crate::ids::Sym, crate::ids::ParamId)>,
	/// The derived generic parameters of the function being declared
	/// (`Analysis::derived_params`), added to its generics after it.
	pub(crate) pending_derived: Vec<crate::ids::ParamId>,
	/// The blocks whose hoisted declarations are declared (a block is walked
	/// again in loop rounds: its statements are scanned once).
	pub(crate) hoisted_blocks: rustc_hash::FxHashSet<NodeId>,
	/// The place each `use` alias and `for` variable over a collection designates.
	pub(crate) aliases: FxHashMap<SymbolId, PlaceId>,
	/// The collection each local iterator reads, and whether its items are
	/// the collection's elements (`iterators.rs`).
	pub(crate) iterator_locals: FxHashMap<SymbolId, (PlaceId, bool)>,
	/// The relation fields and their uses (`relations.rs`).
	pub(crate) relations: relations::RelationState,
	/// The places of relation pools iterated by a loop: their loop
	/// variables are targets, elements of another collection.
	pub(crate) relation_pool_places: rustc_hash::FxHashSet<PlaceId>,
	/// The effects of each local function on the places it captures, applied
	/// at its calls (`effects.rs`), by declaration node.
	pub(crate) closure_effects: FxHashMap<NodeId, Vec<effects::EffectEntry>>,
	/// The places the lambdas of the call being checked write (with the
	/// write): their facts aren't valid at the entry of a sibling lambda.
	pub(crate) lambda_entry_writes: Vec<(PlaceId, bool, NodeId)>,
	/// The function-typed parameters each function only calls, by
	/// declaration node (`callbacks.rs`).
	pub(crate) only_called: FxHashMap<NodeId, u32>,
	/// Names read or written, recorded before module aliases become external
	/// resolutions (`unused.rs`). One bit per symbol, filled during the walk.
	pub(crate) used_symbols: Vec<bool>,
	/// The parent of each node of the tree, built on first use.
	pub(crate) parents: std::cell::OnceCell<Vec<Option<NodeId>>>,
	/// The places the lambda arguments of the call being checked write
	/// (`Some` while a call has several lambda arguments).
	pub(crate) lambda_call_writes: Option<Vec<(PlaceId, bool, NodeId)>>,
	/// The callee name of the call being checked (`moves.rs`).
	pub(crate) callee_name_node: Option<NodeId>,
	/// The locals captured by a local function used as a value (`let call =
	/// show`): read whenever it is called, never moved (`moves.rs`).
	pub(crate) pinned_captures: rustc_hash::FxHashSet<SymbolId>,
	pub(crate) callable_reads: callable_values::CallableReads,
	pub(crate) callable_captures: FxHashMap<(NodeId, SymbolId), Vec<callable_values::ReadSource>>,
	pub(crate) capture_places: FxHashMap<(NodeId, SymbolId), Vec<PlaceId>>,
	/// Sources available after written arguments and before call effects.
	pub(crate) callable_calls: FxHashMap<NodeId, crate::flow::CallableBindings>,
	/// The effects written in each function's signature, by lambda node.
	pub(crate) declared_effect_sets: FxHashMap<NodeId, crate::ids::EffectSetId>,
	/// The captured locals each closure writes, with the write (by lambda
	/// node): `REF021` if it escapes.
	pub(crate) closure_writes: FxHashMap<NodeId, Vec<(SymbolId, NodeId)>>,
	pub(crate) unknown_callback_escapes: FxHashMap<NodeId, NodeId>,
	pub(crate) retention_version: u64,
	pub(crate) retention_checked_version: Option<u64>,
	/// The loop and function depths of each local's declaration (`moves.rs`).
	pub(crate) declaration_depths: FxHashMap<SymbolId, (u32, u32)>,
	/// The last use of each local so far (`moves.rs`).
	pub(crate) last_uses: FxHashMap<SymbolId, moves::LastUse>,
	/// The value nodes in a copy position (`moves.rs`).
	pub(crate) copy_sites: rustc_hash::FxHashSet<NodeId>,
	/// Runtime owner reads, including references and local-call captures.
	pub(crate) move_reads: FxHashMap<NodeId, Vec<SymbolId>>,
	/// Places observed at each read, before later `reuse` changes an alias.
	pub(crate) move_places: FxHashMap<NodeId, Vec<PlaceId>>,
	/// Replacements restore ownership at their successful assignment.
	pub(crate) move_writes: FxHashMap<NodeId, PlaceId>,
	/// Explicit operations ending a unique handle's lifetime in place.
	pub(crate) lifetime_ends: FxHashMap<NodeId, PlaceId>,
	/// The providers of the requirements of the call being applied.
	pub(crate) call_providers: Vec<Option<requirements::Provider>>,
	/// The declared type of the callee's effect path being applied (advanced
	/// brief §3.5 step 4): it decides the owning edges a mutation crosses.
	pub(crate) effect_type: Option<TypeId>,
	/// The declared types of the places a call's effects were substituted into.
	pub(crate) effect_types: FxHashMap<PlaceId, TypeId>,
	/// Each `use` declaring again the name of a `use` of its scope: that one.
	pub(crate) previous_reference: FxHashMap<NodeId, SymbolId>,
	/// The module-level values named by effect paths.
	pub(crate) module_values: FxHashMap<crate::ids::DeclKey, SymbolId>,
	/// The parameters a signature declares as mutated although they can't be
	/// (`EFF005`): their writes are not reported again.
	pub(crate) rejected_effect_roots: rustc_hash::FxHashSet<SymbolId>,
	/// The module system: imports, exports, namespaces (`modules.rs`).
	pub(crate) modules: modules::ModuleState,
	/// The compile-time constructs evaluated already (`comptime.rs`): an
	/// `@` call, an `@if`, an `@for`, an `@test` runs once.
	pub(crate) comptime_evaluated: rustc_hash::FxHashSet<NodeId>,
	/// Inside a `static` value: an error there is a compile error, not one
	/// to handle with `try`.
	pub(crate) comptime_values: u32,
	/// The nesting of type-function applications being computed (`META004`).
	pub(crate) type_function_depth: u32,
	/// The reactive values a `watch` block may write: their facts end with
	/// each statement of the outermost code (`reactivity.rs`).
	pub(crate) watch_writes: rustc_hash::FxHashSet<SymbolId>,
	/// Foreign code: the aliases of `std.c` (`foreign.rs`).
	pub(crate) foreign: foreign::ForeignState,
	/// The `for parallel` loops whose body is being walked (`concurrency.rs`).
	pub(crate) parallel: Vec<concurrency::ParallelFrame>,
	/// The lambdas given to an operation of a parallel view: their bodies
	/// follow the rules of a parallel body (`concurrency.rs`).
	pub(crate) parallel_callbacks: rustc_hash::FxHashSet<NodeId>,
	/// The `parallel()` members resolved, and those an operation follows.
	pub(crate) parallel_views: Vec<(NodeId, bool)>,
	/// Async: awaited and started calls, summaries (`tasks.rs`).
	pub(crate) tasks: tasks::TaskState,
	/// The effect being applied only adds elements (`push`, `insert`): the
	/// relations `i < c.length` stay true (`effects.rs`).
	pub(crate) growing_only: bool,
	/// The places a call changes without replacing them (the receiver of a
	/// method, the argument of a `mutates` parameter): they keep their
	/// narrowing, a change of their payload can't make them `none` or
	/// another member (`effects.rs`).
	pub(crate) kept_narrowings: Vec<PlaceId>,
	/// The fields of types of other files whose default value isn't a
	/// constant expression (`objects.rs`): a literal must give them.
	pub(crate) opaque_defaults: rustc_hash::FxHashSet<(TypeId, crate::ids::Sym)>,
	/// Per local holding a worker pool: whether it never has a `timeout`
	/// (`concurrency.rs`, `WorkerPool.submit`).
	pub(crate) untimed_pools: FxHashMap<SymbolId, bool>,
	/// The names read as the target of an access (`holder` in `holder.task`):
	/// not a use of the whole value (a field moved out, foreign.rs).
	pub(crate) access_roots: rustc_hash::FxHashSet<NodeId>,
	/// The names of `std` the file brings with `use` (`std_library.rs`).
	pub(crate) std_library: std_library::StdState,
	/// The analysis' own diagnostics, sorted at the end.
	output: Vec<Diagnostic>,
	/// The spans of the parse errors: a statement touching one is walked quietly.
	parse_errors: Vec<SourceSpan>,
}

/// Messages are written "a `{}`" whatever the type: "an `Integer`", "an
/// `Array(…)`" before a vowel (but "a `UInt8`").
fn with_articles(message: String) -> String {
	if !message.contains("a `") && !message.contains("A `") {
		return message;
	}
	let mut out = String::with_capacity(message.len() + 4);
	let mut rest = message.as_str();
	while let Some(found) = rest.find(['a', 'A']) {
		let (before, after) = rest.split_at(found);
		out.push_str(before);
		let starts_word = out
			.chars()
			.last()
			.is_none_or(|c| c.is_whitespace() || c == '(');
		let vowel = after
			.strip_prefix(['a', 'A'])
			.and_then(|tail| tail.strip_prefix(" `"))
			.and_then(|tail| tail.chars().next())
			.is_some_and(|c| matches!(c, 'A' | 'E' | 'I' | 'O'));
		out.push_str(&after[..1]);
		if starts_word && vowel {
			out.push('n');
		}
		rest = &after[1..];
	}
	out.push_str(rest);
	out
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
		pool_state: None,
		readonly_elements: rustc_hash::FxHashSet::default(),
		rec_references: FxHashMap::default(),
		error_payloads: Vec::new(),
		view_sink: false,
		written_decls: FxHashMap::default(),
		written_type_keys: FxHashMap::default(),
		written_origin_stack: rustc_hash::FxHashSet::default(),
		receiver_decl: None,
		receiver_origin: None,
		lambda_requirements: FxHashMap::default(),
		pending_implicit: Vec::new(),
		pending_derived: Vec::new(),
		hoisted_blocks: rustc_hash::FxHashSet::default(),
		generic_params: FxHashMap::default(),
		rigid_params: Vec::new(),
		parameter_type: false,
		schema_logic: rustc_hash::FxHashSet::default(),
		deferred_comptime: 0,
		schema_identity: None,
		in_type_function_code: false,
		constraint_depth: 0,
		builtin_params: FxHashMap::default(),
		instance_bindings: FxHashMap::default(),
		implicit_writes: FxHashMap::default(),
		implicit_write_calls: Vec::new(),
		key_sets: key_sets::KeySetState::default(),
		aliases: FxHashMap::default(),
		foreign: foreign::ForeignState::default(),
		std_library: std_library::StdState::default(),
		growing_only: false,
		kept_narrowings: Vec::new(),
		opaque_defaults: rustc_hash::FxHashSet::default(),
		untimed_pools: FxHashMap::default(),
		access_roots: rustc_hash::FxHashSet::default(),
		iterator_locals: FxHashMap::default(),
		relations: relations::RelationState::default(),
		relation_pool_places: rustc_hash::FxHashSet::default(),
		closure_effects: FxHashMap::default(),
		lambda_entry_writes: Vec::new(),
		only_called: FxHashMap::default(),
		used_symbols: Vec::new(),
		parents: std::cell::OnceCell::new(),
		lambda_call_writes: None,
		callee_name_node: None,
		pinned_captures: rustc_hash::FxHashSet::default(),
		callable_reads: FxHashMap::default(),
		callable_captures: FxHashMap::default(),
		capture_places: FxHashMap::default(),
		callable_calls: FxHashMap::default(),
		parallel: Vec::new(),
		parallel_callbacks: rustc_hash::FxHashSet::default(),
		parallel_views: Vec::new(),
		tasks: tasks::TaskState::default(),
		declared_effect_sets: FxHashMap::default(),
		module_values: FxHashMap::default(),
		previous_reference: FxHashMap::default(),
		effect_type: None,
		call_providers: Vec::new(),
		declaration_depths: FxHashMap::default(),
		last_uses: FxHashMap::default(),
		copy_sites: rustc_hash::FxHashSet::default(),
		move_reads: FxHashMap::default(),
		move_places: FxHashMap::default(),
		move_writes: FxHashMap::default(),
		lifetime_ends: FxHashMap::default(),
		effect_types: FxHashMap::default(),
		closure_writes: FxHashMap::default(),
		unknown_callback_escapes: FxHashMap::default(),
		retention_version: 0,
		retention_checked_version: None,
		rejected_effect_roots: rustc_hash::FxHashSet::default(),
		modules: modules::ModuleState::default(),
		comptime_evaluated: rustc_hash::FxHashSet::default(),
		comptime_values: 0,
		type_function_depth: 0,
		watch_writes: rustc_hash::FxHashSet::default(),
		output: Vec::new(),
		parse_errors,
	};
	if root.index() < walker.tree.nodes.len() {
		walker.start_modules(root);
		walker.check_stmt(root);
		walker.finish_modules(root);
		// the async summaries, settled (tasks.rs)
		walker.finish_tasks(root);
		// the fields written into copies of `Type<parameter>` (structural.rs)
		walker.finish_implicit_writes();
		// Error payloads cannot own unique values (errors.rs).
		walker.finish_error_payloads();
		// the API of a C library (c_library.rs)
		walker.finish_c_library(root);
	}
	// the relations of the file (relations.rs)
	walker.finish_relations();
	// the last uses that are copies are moves (moves.rs)
	walker.finish_moves();
	// Demand-time code precedes these final ownership facts.
	walker.s.comptime.invalidate_functions();
	// the static sets of reactivity, and its tables (reactivity.rs)
	walker.finish_reactivity();
	// the values never used (unused.rs)
	walker.report_unused();

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
			View::Name(name) => {
				let ty = self.check_name(n, name);
				self.check_foreign_value(n, ty);
				ty
			}
			View::ImplicitMember(name) => self.check_implicit_member(n, name, expected),
			View::Group(inner) => self.check_group(n, inner, expected),
			View::Copy(inner) => self.check_copy(n, inner, expected),
			View::EmptyGroup => self.check_empty_group(n),
			View::Array { items } => self.check_array(n, items, expected),
			View::Tuple { items } => self.check_tuple(n, items, expected),
			View::Spread(value) => self.check_spread(n, value),
			View::Range {
				start,
				end,
				inclusive,
			} => self.check_range(n, start, end, inclusive),
			View::Access { target, members } => {
				let ty = self.check_access(n, target, members);
				self.check_foreign_value(n, ty);
				ty
			}
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
			// a generic lambda (phase 2 of "Generic functions carry their list on the function value", functions.rs)
			View::Lambda { signature, body } if !signature.generics.is_empty() => {
				self.check_generic_lambda(n, signature, body, expected)
			}
			// a lambda written without types where a generic function type is expected
			View::Lambda { signature, body }
				if self.takes_expected_generics(signature, expected) =>
			{
				self.check_lambda_of_generic_type(n, signature, body, expected)
			}
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
			View::Watch(body) => self.check_watch(n, body),

			// async (tasks.rs)
			View::Await(value) => self.check_await(n, value, expected),
			View::Start(value) => self.check_start(n, value, expected),

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
			View::Object { items } => {
				let ty = self.check_object(n, items, expected);
				self.check_stored_lambdas(items);
				ty
			}
			View::TypedObject { target, members } => {
				let ty = self.check_typed_object(n, target, members);
				self.check_stored_lambdas(members);
				ty
			}
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
		if matches!(self.view(n), View::Index { .. }) {
			self.note_place_read(n);
		}
		if let Some(slot) = self.a.types.get_mut(n.index())
			&& slot.is_unset()
		{
			*slot = ty;
		}
		if ty == TypeId::NEVER {
			// the code after a `Never` expression is unreachable
			self.body.flow.set_unreachable();
		}
		self.record_callable_read(n, ty);
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
		if !broken {
			self.warn_pure_statement(n);
		}
		// the `watch` blocks may run after it (reactivity.rs)
		self.after_outer_statement();
	}

	/// `W-DO-1`: a statement whose value is dropped and whose computation
	/// has no effect (`a + 1`). The last value of a function body without
	/// return type is `WFN001` (functions.rs).
	fn warn_pure_statement(&mut self, n: NodeId) {
		if !self.is_value_statement(n) || !self.is_pure_expression(n) {
			return;
		}
		let ty = self.type_of(n);
		if matches!(ty, TypeId::VOID | TypeId::NEVER)
			|| ty.is_unset()
			|| self.s.types.is_recovering(ty)
		{
			return;
		}
		// a statement of a block or of the module (not an arm's value)
		let Some(parent) = self.parent(n) else {
			return;
		};
		if !matches!(self.view(parent), View::Block { .. } | View::Module { .. }) {
			return;
		}
		if let View::Block { statements } = self.view(parent)
			&& statements.last() == Some(n)
			&& self.body.functions.last().is_some_and(|cx| {
				self.a
					.functions
					.get(&cx.node)
					.is_some_and(|info| info.body == parent)
			}) {
			return;
		}
		if !self.reported_once.insert((n, "W-DO-1")) {
			return;
		}
		let text = self.tree.text(n.index()).to_string();
		let diagnostic = self.diagnostic(
			&codes::values::W_DO_1,
			n,
			format!("`{text}` computes a value and drops it: this statement has no effect. Remove it, or use its value"),
		);
		self.report(diagnostic);
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
					// a `use` alias or a `for` variable writes the place it designates
					Resolution::Local(symbol) if self.is_alias(symbol) => {
						self.alias_writable(symbol)
					}
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
		// a generic function declaration given where a generic function type is
		// expected: a generic function value (generics.rs)
		let actual = self
			.generic_declaration_value(n, actual, target)
			.unwrap_or(actual);
		// a generic function given where a function type is expected (generics.rs)
		let actual = match self.instantiate_function_value(n, actual, target) {
			Some(instance) if self.s.types.is_recovering(instance) => return,
			Some(instance) => instance,
			None => actual,
		};
		match self.s.types.assignable(actual, target, Mode::Copy) {
			Ok(()) => {
				self.check_function_projection_owner(n, actual, target);
				self.check_structure_conversion(n, actual, target);
				// a `Character` stored as a `String` is converted
				if self.s.types.kind(actual) == TypeKind::Character
					&& self.s.types.kind(target) == TypeKind::String
				{
					self.a.coercions.insert(n, target);
				}
				self.sink_union_member(n, actual, target);
				// a copy into a narrower object type (structural.rs)
				self.record_narrowing_copy(n, actual, target);
			}
			Err(Reason::Range) => self.sink_integer(n, actual, target),
			// `Array<Int64>` for `Array<Integer>`: the same values, stored
			// differently (types brief D2)
			Err(Reason::Storage) => {
				let message = format!(
					"a `{}` is not a `{}`: the same values are stored differently inside them (a 64-bit integer is a `bigint` on JavaScript, an `Integer` a number), convert the items, ex: `values.map((value) => Integer(value))`",
					self.display(actual),
					self.display(target)
				);
				let diagnostic = self.diagnostic(&codes::types::CV005, n, message);
				self.report_flow(diagnostic);
			}
			Err(_) if self.string_sink_mismatch(n, actual, target) => {}
			Err(_) if self.inexact_collection(n, actual, target) => {}
			Err(_) if self.nominal_mismatch(n, actual, target) => {}
			// an `async` function where the type isn't `async` (tasks.rs)
			Err(_) if self.async_mismatch(n, actual, target) => {}
			// a generic function type expected (generics.rs)
			Err(_) if let Some(diagnostic) = self.generic_function_problem(n, actual, target) => {
				self.report_flow(diagnostic);
			}
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
				self.report_missing_field(n, actual, target);
			}
			Err(_) if self.optional_sink(n, actual, target) => {}
			// `let f: Float32 = 16_777_217`, `let p: Percentage = 50` (math.rs)
			Err(_) if self.number_literal_sink(n, actual, target) => {}
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
				matches!(
					self.s.types.kind(self.s.types.unfold(member)),
					TypeKind::Object(_)
				) && self.s.types.assignable(actual, member, Mode::Copy).is_ok()
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
	pub fn report(&mut self, mut diagnostic: Diagnostic) {
		// the notes follow the rule of the messages ("an `Integer`")
		for (_, note) in &mut diagnostic.related {
			*note = with_articles(std::mem::take(note));
		}
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
		let target = match self.body.flow_buffers.last_mut() {
			Some(buffer) => buffer,
			None => &mut self.output,
		};
		// one operation with two effects (`pool.delete`: the element and the
		// size), or a node read twice by the walk, breaks a rule once
		let repeated = target.iter().rev().take(16).any(|last| {
			std::ptr::eq(last.code, diagnostic.code) && last.location == diagnostic.location
		});
		if !repeated {
			target.push(diagnostic);
		}
	}

	fn count_unsupported(&mut self, diagnostic: &Diagnostic) {
		if std::ptr::eq(diagnostic.code, &codes::analyzer::AN001) {
			self.unsupported_seen += 1;
		}
	}

	/// The parent of a node (`None` for the root), from a table built once.
	pub(crate) fn parent(&self, n: NodeId) -> Option<NodeId> {
		let parents = self.parents.get_or_init(|| {
			let count = self.tree.nodes.len();
			let mut parents: Vec<Option<NodeId>> = vec![None; count];
			for index in 0..count {
				let node = NodeId::new(index);
				for child in syntax::children(&self.tree, node) {
					if let Some(slot) = parents.get_mut(child.index()) {
						*slot = Some(node);
					}
				}
			}
			parents
		});
		parents.get(n.index()).copied().flatten()
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
			with_articles(message.into()),
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
	pub(crate) fn hoisted(&mut self, n: NodeId, f: impl FnOnce(&mut Self) -> TypeId) -> TypeId {
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
		let symbol_data = self.a.symbols.symbol_mut(symbol);
		symbol_data.state = DeclState::Done;
		if symbol_data.ty.is_unset() {
			symbol_data.ty = TypeId::UNKNOWN;
		}
		// other files may read it while this one is analyzed (modules.rs)
		self.publish(symbol);
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
		if let Resolution::Local(symbol) = resolution
			&& !self
				.used_symbols
				.get(symbol.index())
				.copied()
				.unwrap_or(false)
			&& matches!(self.view(n), View::Name(_))
			&& self.span(n) != self.a.symbols.symbol(symbol).name_span
		{
			if self.used_symbols.len() <= symbol.index() {
				self.used_symbols.resize(symbol.index() + 1, false);
			}
			self.used_symbols[symbol.index()] = true;
		}
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
