# fa-analyzer: implementation design

Status: accepted by the project lead. It builds on `design/compiler-architecture.md`, which is binding, and on the three briefs, cited as "types brief", "values brief" and "advanced brief". Where the briefs leave a choice open, this document makes it. These choices are listed in §0.

## 0. Decisions taken here

| # | Decision | Why |
|---|---|---|
| A1 | **One `check` function per node kind, split into a memoized *type part* and an always-run *flow part*.** Loop and SCC replays re-enter the same functions. The type part is skipped because `types[node]` is already set. | One traversal is written once. It satisfies "types memoized, flow re-run" without a second walker. |
| A2 | **Typing that depended on narrowing records a *narrowing requirement* on the node.** The flow part re-validates it at every visit. | The types are memoized from iteration 1, but a narrowing lost at the fixpoint must still produce E-OPT-1 / E-NAR-1 (T-NAR-4, T-NAR-9). |
| A3 | **Flow diagnostics are buffered per loop iteration and per SCC round. Only the last, stable round is kept.** Type diagnostics are emitted inside the memoized part, so they come out once by construction. | Advanced brief §2.2. |
| A4 | **Reference checking is part of the flow state of the same walk.** This overrides the types brief §1 ("a second walk"). | Architecture doc. Advanced brief §2.1, D1. |
| A5 | **The analyzer reads the parser through a view layer, `syntax.rs`.** It is the only module that matches on `fa_parser::Node`. The analyzer computes block hoisting itself: one scan of the block's statements, never of bodies. | The parser is changing now, and today it hoists only `function`, not `type`/`namespace`. |
| A6 | **Diagnostic codes come from one registry: one canonical code plus aliases.** The canonical code belongs to the brief that owns the area: values brief → `E-*`/`W-*`; types brief → `TY/GN/TF/CV/FN/CL`; advanced brief → `EFF/REF/MOD/REQ/…`. Exact duplicates become aliases, for example `EF015`→`E-ERR-3`, `EF030`→`REF013`, `E-DECL-16`→`REF001`, `FN010`→`EFF020`, `CV001`→`E-NUM-11`, `CL060`→`E-NUM-17`, `WEF002`→`W-ERR-1`, `E-TYPE-2`→`TY040`. Codes owned by the analyzer itself are `AN0xx`, for example `AN001` "construct not supported yet". | Brief tests can be pasted verbatim, and the output has one stable code per condition. |
| A7 | **The `Session` is single-threaded in v1.** Files are parsed in parallel, but analysis runs on one thread with `&mut TypeStore`. The sharded interner of types brief §2.1 is an internal upgrade of `types/store.rs`, kept for later. | Cross-module on-demand analysis is sequential anyway. A file is analyzed in milliseconds. |
| A8 | **Intervals use `ethnum::I256`, saturating at ±2^250, which stands for ±∞.** | Covers `UInt128` products without overflow (values brief §1, types brief §2.2). |
| A9 | **A flow state is a sorted `SmallVec<(PlaceId, FactId)>` of hash-consed facts.** There is no persistent-map crate. | A fork is a memcpy of about 8 entries. A join is a merge. Fixpoint equality is a slice compare. |
| A10 | **Nominal identity is a `DeclKey` = (module path, names of enclosing declarations, disambiguator).** It is never a node index. | Ids stay stable across re-parses, so dependents that are not re-analyzed (unchanged interface hash) still compare equal. |
| A11 | **A named local `function` body sees no facts about the places it captures.** Only lambdas see the narrowings present at their creation (values brief D-102). | Hoisting means the body may be analyzed before its declaration point. This keeps the result independent of order. |
| A12 | **The prelude is Rust tables (`builtins/`) in M1. From M2 it is Fa sources in `packages/std` with `intrinsics.primitive`**, parsed once per session (advanced brief §22.3). | Lets the backends start without the std library work. |
| A13 | **Value-initializer cycles found on demand are `MOD007`.** Recursion without a declared return type is `EFF020` (alias `FN010`). | Advanced brief §15.7 and §2.4. |

## 1. Crate layout and public API

`packages/analyzer` (crate `fa-analyzer`) moves from `exclude` to `members` in the root `Cargo.toml`.

- Dependencies: `fa-parser`, `rustc-hash`, `smallvec`, `ethnum`, `bitflags`. No `tower-lsp`.
- `fa-codebase-parser` is used only for its plain-data `NameTable`, behind the `ModuleResolver` trait.

```
src/
  lib.rs               re-exports: Session, Analysis, Program, ids, Diagnostic, queries
  ids.rs               NodeId, FileId, ModuleId, SymbolId, Sym, TypeId, ListId, DeclKey, … (u32 newtypes)
  intern.rs            Sym interner (owns its bytes; does not rely on the parser's &'static str)
  syntax.rs            View layer over fa_parser::Node: View enum, children(), literal decoding,
                       block declarations (hoisting scan), n-ary → binary folding, Power right-assoc
  session.rs           Session: files, modules, TypeStore, prelude, resolver, analyze_*, incremental update
  analysis.rs          Analysis (per-file side tables) and backend-facing accessors
  ide.rs               node_at, hover, definition, completions, display helpers (LSP-agnostic)
  diagnostics/
    mod.rs             Diagnostic, Severity, Location, Related, sorting, rendering for the CLI
    codes/{values,types,advanced,analyzer}.rs   CodeInfo statics (one file per brief, no merge conflicts)
  types/
    mod.rs             TypeId, TypeKind, CollKind, IntStorage, FloatBits
    store.rs           TypeStore: interning, predefined ids, per-type flags, stable hashes
    int.rs             IntType, Interval, interval arithmetic (values brief §6.5), compute types (§6.7)
    union.rs           normalize_union (types brief §3.4.1), distinguishability (§3.4.2)
    relate.rs          assignable(S, T, Mode) with cache, join, literal adaptation
    subst.rs           substitution, generic application, instantiation lists
    shape.rs           object shapes, NominalInfo, members/methods/defaults (TypeDeclInfo)
    recursive.rs       recursive SCC canonicalization (M2)
    display.rs         type printing with spellings (hover, messages)
  symbols.rs           SymbolTable, Symbol, scopes arena, Resolution
  builtins/            prelude: scalars.rs, numbers.rs (conversions, methods), strings.rs,
                       console.rs, errors.rs (IndexError…, values brief D-25/types brief D50), array.rs
  check/               THE WALK
    mod.rs             Walker, check()/check_stmt() dispatch over View, memo(), report(), contexts
    decl.rs            let/mutable/static/type/namespace, hoisting, on-demand analysis, cycles
    type_expr.rs       type-mode resolution of expression nodes
    expr.rs            literals, names, access, interpolation, groups
    numbers.rs         arithmetic trees, sinks, overflow/range bits, conversions, divisions
    ops.rs             comparison, equality, is, logic, ??, ?.
    control.rs         if/when/do/loops/break/continue/return; fork/join; fixpoint driver
    functions.rs       declarations, lambdas, signatures, summaries, SCC rounds
    calls.rs           argument binding, generic inference, effect application at calls
    objects.rs         object literals, typed literals, fields, spreads, .=
    patterns.rs        when patterns, exhaustiveness, destructuring
    errors.rs          throws/try/catch/throw coverage
    tests.rs           test blocks, assert
    effects.rs         (M2) path building, accumulation, coverage
    references.rs      (M2) use/mutable use/reuse, ref transfer, alias & iteration rules
  flow/
    mod.rs             FlowState, fork/join/widen, PlaceId trie
    facts.rs           Fact, FactStore, interval sets with exclusions (values brief D-95)
    narrowing.rs       condition → (true state, false state)
    refs.rs            (M2) RefState, Identity, at_or_below / overlap / mutation_invalidates
  effects.rs           (M2) Path, EffectSet, normalization, covers()
  summary.rs           FnSummary, interface hash
  lower/when.rs        shared desugaring for backends: decision list of a `when`
tests/
  cases.rs             data-driven harness
  cases/<brief>/<section>/*.fa
```

### Public API

```rust
pub struct Session { /* files, modules, types: TypeStore, symbols per file, prelude, resolver, cancel flag */ }

impl Session {
	pub fn new(options: Options) -> Self;                               // Options { prelude: PreludeSource }
	pub fn set_resolver(&mut self, resolver: Box<dyn ModuleResolver>);  // name tables (advanced brief §15.3)
	pub fn set_source(&mut self, module: ModulePath, text: Arc<str>) -> FileId; // parse; new FileId per version
	pub fn analyze_file(&mut self, file: FileId) -> &Analysis;          // whole file, memoized
	pub fn analyze_program(&mut self, entry: ModulePath) -> Program<'_>;// all files of the roots (D42)
	pub fn update(&mut self, module: ModulePath, text: Arc<str>) -> Vec<FileId>; // LSP: re-analyzed files
	pub fn cancel_flag(&self) -> Arc<AtomicBool>;
	// queries (ide.rs)
	pub fn node_at(&self, file: FileId, offset: u32) -> Option<NodeId>;        // innermost span
	pub fn definition(&self, file: FileId, offset: u32) -> Option<Location>;
	pub fn hover(&self, file: FileId, offset: u32) -> Option<Hover>;           // { span, markdown }
	pub fn completions(&self, file: FileId, offset: u32) -> Vec<Completion>;   // { label, kind, detail }
	pub fn display_type(&self, ty: TypeId) -> String;
	pub fn tree(&self, file: FileId) -> &TypedSyntaxTree;
	pub fn types(&self) -> &TypeStore;
}

pub fn analyze_source(text: &str) -> (Session, FileId);               // standalone file: tests, `fa run x.fa`

pub trait ModuleResolver {                                            // implemented by the CLI / LSP over fa-codebase-parser
	fn lookup(&self, from: ModulePath, name: &str) -> Lookup;         // Sibling | RootEntry | Dependency | Folder | Miss
}

pub struct Program<'s> {                                              // what backends receive
	pub session: &'s Session,
	pub files: Vec<FileId>,
	pub entry: Option<FileId>,
	pub init_order: Vec<DeclKey>,                                     // module-level values (advanced brief §15.7)
}
```

`Analysis` holds the per-file side tables (§2.6). Backends and the language server query it through `session.analysis(file)`:

- **Type of a node**: `analysis.type_of(n)`. For a type-expression node (flag `TYPE_EXPR`), this is the type it denotes.
- **Symbol of an identifier**: `analysis.resolution(n)`.
- **Go-to-definition**: resolution → `Symbol.name_span`. For `Resolution::Module(m, name)`, look up the current file of `m` and the symbol by name, so a stale `FileId` is never followed.
- **Hover**: the declaration keyword, the name, the narrowed type of that read, and the interval fact when it is narrower than the type (for example `count: Integer — here 0..=10`).
- **Completions**: walk the recorded scope chain at the offset (symbols visible before the offset, plus hoisted ones), then module scope, siblings and prelude. After `x.`, list the fields and methods of `type_of(x)`. This needs the parser to recover `x.` as `Access` with a missing member (§7).

## 2. Data structures

### 2.1 Ids

All ids are `#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)] struct X(u32)`.

| Id | Meaning |
|---|---|
| `NodeId` | Index into `TypedSyntaxTree::nodes`, converted from `usize` in `syntax.rs`. |
| `FileId` | One parse of one file. A re-parse gets a new id. |
| `ModuleId` / `ModulePath` | Interned module path, stable for the session. |
| `SymbolId` | Local to a file's `SymbolTable`. |
| `Sym` | Interned identifier or string. |
| `DeclKey` | Interned `(ModuleId, ListId<Sym> enclosing names, u16 disambiguator)`. Nominal identity, summaries, requirement keys. |
| `TypeId` | Index 0 = `TypeId::UNSET` (no type yet / not an expression). Then the predefined ids. |
| `ListId`, `ShapeId`, `SigId`, `PathId`, `EffectSetId`, `NominalId`, `IntId`, `MaskId`, `ParamId`, `ConstId`, `RecId`, `SpellingId` | Indices into hash-consed side tables of `TypeStore`. |
| `PlaceId`, `FactId`, `IntervalId` | Per-file flow stores. |

### 2.2 Types

This takes types brief §2.2 almost verbatim. The changes: `Unknown` is the error-recovery type, and nominal identity goes through `DeclKey`.

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum TypeKind {                       // ≤ 16 bytes; variable data lives in side tables
	Never, Void, None, ErrorTop, Unknown,  // Unknown: recovery, satisfies/accepts everything, never re-reported
	Boolean, BoolLit(bool),
	Int(IntId),                            // IntType { min: I256, max: I256, storage: IntStorage }; literal: min == max
	SymInt(SymIntId),                      // bounds affine in value params, generic bodies only (M2)
	Float(FloatBits), FloatLit(FloatBits, ConstId),
	BigInt, BigIntLit(ConstId), Percentage,
	String, StringLit(Sym), Character,
	Object(ShapeId),                       // Shape { fields: [FieldEntry] sorted by name, nominal: Option<NominalId> }
	Tuple(ListId),
	Union(ListId),                         // anonymous, normalized, ≥ 2 members, sorted by TypeId; `T?` = Union[T, NONE]
	NamedUnion(NominalId, MaskId), Member(NominalId, u16),   // Union / Errors types, masks for differences and narrowing
	Enum(NominalId, MaskId), EnumMember(NominalId, u16),
	Fields(FieldsShapeId), CStruct(NominalId),
	Function(SigId),
	Collection(CollKind, TypeId, TypeId),  // item, key (NONE except maps)
	Iterator(TypeId, PathId),
	Ref(TypeId, PathId),                   // `T in path`
	Range, Task(TypeId, TypeId),
	Param(ParamId), ImplicitParam(DeclKey, u16), Const(ConstId), Rec(RecId),
}

pub enum IntStorage { Default, I8, I16, I32, I64, I128, U8, U16, U32, U64, U128 } // Integer = Default, Int64 = I64 (types brief D2)
pub struct NominalInfo { key: DeclKey, args: ListId, kind: NominalKind /* Union|Errors|Enum|CStruct|Private */ }
pub struct FnSig {                                         // interned → SigId
	params: ParamListId,                                   // ParamSig { ty: TypeId, variadic: bool }
	ret: TypeId, throws: TypeId /* NEVER = none */, requires: ReqListId, effects: EffectSetId,
	stored: u32,                                           // bit i: function-typed param i escapes (advanced brief §5.11)
	is_async: bool,
}
pub struct ParamInfo { owner: DeclKey, index: u16, name: Sym, constraint: TypeId, default: TypeId }

pub struct TypeStore {
	kinds: Vec<TypeKind>, flags: Vec<TypeFlags>, stable: Vec<u128>,   // per TypeId
	map: FxHashMap<TypeKind, TypeId>,
	lists: ListArena,                       // one Vec<TypeId>, (start, len) interned by content
	ints: Interner<IntType>, shapes: Interner<Shape>, sigs: Interner<FnSig>, paths: Interner<Path>,
	effects: Interner<EffectSet>, masks: Interner<Mask>, consts: Interner<ConstValue>,
	nominals: Interner<NominalInfo>, decl_info: FxHashMap<NominalId, TypeDeclInfo>, // members, methods, defaults (replaced on re-analysis)
	params: Vec<ParamInfo>, spellings: Interner<Spelling>,
	assign_cache: FxHashMap<(TypeId, TypeId, Mode), Result<(), Reason>>,
	subst_cache: FxHashMap<(TypeId, ListId), TypeId>,
}
```

Rules:

- **Predefined ids are interned first, at fixed indices**: `NEVER`, `VOID`, `NONE`, `ERROR`, `UNKNOWN`, `BOOLEAN`, `TRUE`, `FALSE`, `INTEGER`, `INT8…UINT128`, `INT64`, `BYTE` (= `UINT8`), `FLOAT`, `FLOAT32`, `BIG_INT`, `PERCENTAGE`, `STRING`, `CHARACTER`, `RANGE`.
- **Flags** are computed at interning (types brief §2.3): `HAS_PARAM`, `FLAT`, `TRIVIALLY_COPYABLE`, `COMPARABLE`, `ORDERED`, `MOVE_ONLY`, `HAS_VOID`, `RECURSIVE`, `KIND:4`, `JS_BIGINT`, plus `HAS_UNKNOWN`, which suppresses cascading diagnostics.
- **Unions are normalized** by `union.rs` on every `|`, join, difference and substitution: flatten, drop `Never`, non-object absorption (types brief D1), merge touching ranges of the same storage, mask members of one nominal union, then sort and dedupe. `TY040` is checked where a union is written or joined without an expected type.
- **Literal types** exist for annotations, `static`, `type` aliases and members. Unannotated bindings get the widened type, and the exact value is a fact (values brief §2.1).
- **Equality is `TypeId ==`.** `assignable(S, T, Mode)` with `Mode = Read | Copy | Write(PathSetId) | Exact` is cached, and coinductive for `Rec`.
- **Spellings** (`SpellingId` = name + args as written + the declaration) are kept per type-expression node and per symbol, for messages, hover and method resolution (types brief §2.2).

### 2.3 Symbols and scopes

```rust
pub struct Symbol {
	pub name: Sym, pub kind: SymbolKind, pub decl_node: NodeId, pub name_span: SourceSpan,
	pub scope: ScopeId, pub ty: TypeId,            // declared or inferred; UNSET while pending
	pub spelling: SpellingId, pub state: DeclState, pub key: Option<DeclKey>, // key: module-level, types, functions
	pub summary: Option<SummaryId>,
}
pub enum SymbolKind { Let, Mutable, Use, MutableUse, Static, Parameter { index: u16 }, SelfParameter,
	LoopVariable { copy: bool }, CatchBinding, PatternBinding, Function, Type, TypeParameter, Namespace,
	Test, Reactive, Derived, NameAlias, Implicit /* `arguments`, values brief D4 */ }
pub enum DeclState { Sequential, Pending, Signature { frame: u32 }, Body { frame: u32 }, Done }
pub struct Scope { parent: Option<ScopeId>, owner: NodeId, kind: ScopeKind, first: u32, len: u32 /* into a flat Vec<SymbolId> */ }
pub enum Resolution { Unresolved, Local(SymbolId), Module(ModuleId, Sym), Builtin(BuiltinId),
	Field(ShapeId, u16), Method(DeclKey, Sym), Member(NominalId, u16) }
```

- **Lookup**: a linear scan of each scope, since local scopes have under 16 symbols. The module scope also has an `FxHashMap<Sym, SymbolId>`. Then come siblings, root entries and dependencies through `ModuleResolver`, then the prelude. Every external lookup, including misses, is recorded in the file's dependency list (advanced brief §21.2).
- **Same-scope redeclaration** is `E-DECL-2`. The only exception is `use` over an invalidated `use` (`REF003` rules, M2).
- **Scopes are an arena, kept in the `Analysis`.** Completions and on-demand analysis can jump to any scope by id.

### 2.4 Flow state

```rust
pub struct PlaceId(u32);   // trie node: Place { root: PlaceRoot, parent: Option<PlaceId>, step: Step }
pub enum PlaceRoot { Symbol(SymbolId), ModuleValue(DeclKey), Temporary(NodeId) }
pub enum Step { Field(Sym), Index(Identity), Range(Identity, Identity, bool), All, Length, Size, Payload(TypeId), Characters, Child }

pub struct Fact {                       // hash-consed in FactStore → FactId; equality = id compare
	pub narrowed: TypeId,               // UNSET = no narrowing; else the member set (a TypeId union/mask)
	pub int: IntervalSetId,             // interval + ≤ 8 excluded intervals (values brief D-95); NONE = type range
	pub literal: ConstId,               // exact literal (strings, booleans, enum members, numbers)
	pub length: IntervalId,             // collection length / string grapheme count
	pub key_present: ConstListId,       // map key proofs (types brief D32)
}

#[derive(Clone, PartialEq)]
pub struct FlowState {
	pub reachable: bool,                                  // false after a Never expression
	pub facts: SmallVec<[(PlaceId, FactId); 8]>,          // sorted by PlaceId; absent = declared type only
	pub refs: SmallVec<[(SymbolId, RefStateId); 4]>,      // M2: reference validity (advanced brief §5.1)
	pub iterating: SmallVec<[PlaceId; 2]>,                // M2: iteration stack (§5.8)
	pub time: u32,                                        // M2: freshness clock
}
pub struct Requirement { node: NodeId, place: PlaceId, needs: TypeId }  // A2: re-checked by the flow part
```

- **Join**: unreachable states are the identity. Otherwise pointwise, keeping only places present in both. Interval hull, intersection of exclusions, union of `narrowed`, literal kept only if equal (values brief D-30).
- **Widening** (values brief D-96): a bound that moved since the previous head becomes ±∞, then is clipped to the type.
- **Writes**:
  - an assignment replaces the facts of the place;
  - `drop_below(p)` removes the facts of `p` and of every descendant (one parent walk per entry);
  - a `mutates p` call drops the facts at and below `p`, and length facts at or above `p`.

### 2.5 Effects and summaries (M2, declared in M1 so signatures are stable)

```rust
pub struct Path { root: PathRoot, steps: StepListId }      // interned in TypeStore → PathId (signatures, Ref owners)
pub enum PathRoot { Param(u16), SelfParam, ModuleValue(DeclKey), Requirement(u16) }
pub struct EffectSet { mutates: SmallVec<[PathId; 2]>, invalidates: SmallVec<[PathId; 2]> } // normalized on interning
pub struct FnSummary {
	pub sig: SigId,                     // declared, or inferred for single-expression bodies
	pub bits: SummaryBits,              // fresh, pure, depends_on_target, reaches_io, may_write_reactive
	pub needs_full_type: u32, pub stored: u32,
	pub module_reads: DeclListId, pub reactive_reads: DeclListId,
	pub state: SummaryState,            // Provisional (in an SCC) | Final
	pub interface_hash: u128,           // from stable hashes, never from ids (advanced brief §21.3)
}
```

### 2.6 Side tables (`Analysis`)

```rust
pub struct Analysis {
	pub file: FileId,
	pub types: Vec<TypeId>,                         // per node (4 B)
	pub resolutions: Vec<Resolution>,               // per node (12 B; dense because most nodes are names or members)
	pub flags: Vec<NodeFlags>,                      // per node (4 B)
	pub symbols: SymbolTable, pub scopes: Vec<Scope>,
	pub arith: FxHashMap<NodeId, ArithInfo>,        // every integer arithmetic node and every checked sink
	pub intervals: FxHashMap<NodeId, IntervalId>,   // integer nodes whose fact is narrower than their type (hover, proofs)
	pub coercions: FxHashMap<NodeId, TypeId>,       // implicit conversion at a sink: union injection, Integer↔Int64, Float32→Float, Character→String
	pub instantiations: FxHashMap<NodeId, ListId>,  // generic args per call / typed literal / generic function value
	pub functions: FxHashMap<NodeId, FnInfo>,       // per function or lambda node: summary, captures, escapes (M2)
	pub spellings: FxHashMap<NodeId, SpellingId>,
	// M2+: places, moves, requirement_args, relation_writes, reactive_*, static_values (advanced brief §2.6)
	pub diagnostics: Vec<Diagnostic>,               // parse diagnostics first, then sorted by span
	pub dependencies: Vec<Dependency>,              // lookups (hits and misses) and interfaces used
}
bitflags! { pub struct NodeFlags: u32 {
	const OVERFLOW_NATIVE  = 1 << 0;  // node may exceed the native compute type (§3.6)
	const OVERFLOW_JS      = 1 << 1;  // number-represented node may exceed ±(2^53−1)
	const SINK_CHECK       = 1 << 2;  // root value must be range-checked against coercions[node] / its sink type
	const DIVISOR_CHECK    = 1 << 3;  // divisor not proven ≠ 0 → DivisionByZero path
	const INDEX_CHECK      = 1 << 4;  // index/slice not proven → IndexError path
	const CONVERSION_CHECK = 1 << 5;  // Int8(x), Integer(f) … not proven → ArithmeticError path
	const EXPONENT_CHECK   = 1 << 6;  const SHIFT_CHECK = 1 << 7;
	const MIN_DIV_GUARD    = 1 << 8;  // INT_MIN // -1, %% -1 guards needed (values brief D-49)
	const NARROWED         = 1 << 9;  // payload access without re-testing the tag
	const UNREACHABLE      = 1 << 10; const TYPE_EXPR = 1 << 11; const CONST_FOLDED = 1 << 12;
	const PLACE            = 1 << 13; const MOVE = 1 << 14 /* M2 */; const CAN_FAIL = 1 << 15;
}}
pub struct ArithInfo { pub interval: IntervalId, pub compute: ComputeType /* Int64 | Int128 | UInt128 */, pub root: bool }
```

### 2.7 Diagnostics

```rust
pub struct CodeInfo { pub id: &'static str, pub aliases: &'static [&'static str], pub severity: Severity, pub summary: &'static str }
pub type Code = &'static CodeInfo;
pub struct Location { pub file: FileId, pub span: SourceSpan }
pub struct Diagnostic { pub severity: Severity, pub code: Code, pub message: String, pub location: Location,
	pub related: Vec<(Location, String)> }
pub enum Severity { Error, Warning, Hint }
```

- **Codes are statics**, one file per brief, generated by a `codes!` macro.
- **A unit test checks that no id or alias string appears twice** across the registry.
- **Cascade suppression**: when an operand has `UNKNOWN` (or the `HAS_UNKNOWN` flag), no further diagnostic is emitted for that expression.
- **Conversion to LSP types** happens in `fa-language-server`.

## 3. The walk

### 3.1 Shape

```rust
pub struct Walker<'s> {
	s: &'s mut Session, file: FileId, tree: Arc<TypedSyntaxTree>,
	body: BodyCx,          // swapped wholesale for on-demand analysis
}
struct BodyCx {
	scope: ScopeId, flow: FlowState,
	functions: Vec<FnCx>,  // effect accumulator, declared summary, return type, body kind, requirements in scope
	loops: Vec<LoopCx>,    // break/continue states, value type of `loop`
	flow_buffers: Vec<Vec<Diagnostic>>,   // A3
	requirements: Vec<Requirement>,       // A2 (indexed per node)
}
impl Walker<'_> {
	fn check(&mut self, n: NodeId, expected: Option<TypeId>) -> TypeId;   // expression in value position
	fn check_stmt(&mut self, n: NodeId);                                   // statement position (if/when are Void)
	fn check_cond(&mut self, n: NodeId) -> (FlowState, FlowState);         // true / false states (narrowing.rs)
	fn check_place(&mut self, n: NodeId) -> PlaceInfo;                     // assignment targets, `use`, effect args
	fn sink(&mut self, n: NodeId, actual: TypeId, target: TypeId);         // values brief §2.3: assignability, ranges, coercion
	fn memo(&mut self, n: NodeId, f: impl FnOnce(&mut Self) -> TypeId) -> TypeId; // runs f iff types[n] == UNSET
	fn report(&mut self, d: Diagnostic);       // type part: straight to output
	fn report_flow(&mut self, d: Diagnostic);  // flow part: top flow buffer, or output if none
}
```

`check` matches exhaustively on `syntax::View`, which belongs to the analyzer. `View::Unsupported(kind)` produces `AN001` once, then type `UNKNOWN`, and its children are still walked so names resolve for the LSP. `View::Error` (a parser error node) produces `UNKNOWN` with no diagnostic. Every handler follows the same pattern:

```
children: self.check(child, expected_for_child)…   // every visit (they carry flow)
ty = self.memo(n, |s| type rule + type diagnostics + record requirements/instantiations/coercions)
flow transfer of n: facts, intervals, check bits (set *and cleared*), requirement re-validation, report_flow
```

### 3.2 Blocks, hoisting, on-demand declarations

- **Entering a block** opens a scope.
- **Declarations found by `syntax::block_declarations`** are declared `Pending`: `function`, `type`, `namespace` anywhere, and every declaration at the module level of a package file. Ordinary local `let`/`mutable` are declared `Sequential` when reached.
- **Statements are walked in order.** When reached in order, a pending declaration is analyzed then, unless it is already `Done`.
- **`symbol_type(sym, use_site)` by state**:
  - `Done` / `Sequential`: return `ty`.
  - `Pending`: save `BodyCx`. Set a fresh `BodyCx` whose scope is the declaration's scope, whose flow state is empty (A11), and whose buffer stack is empty. Run `analyze_decl`, then restore. A declaration in another module goes through `Walker { s: &mut *self.s, file: other, … }`, the same way.
  - `Signature{frame}` (cycle while resolving a signature or a type):
    - a type reference → recursive type placeholder (M2 §3.6 of the types brief; until then `AN001`);
    - an unannotated value → `MOD007`;
    - otherwise `UNKNOWN`.
  - `Body{frame}`: the signature is available. Mark the SCC (§3.4) and return the function type with its current provisional effects. If the signature has no declared return type: `EFF020`, `UNKNOWN`.
- **A function is analyzed in two phases**:
  1. The signature: parameters, declared return type, `throws`, `requires`, `mutates`/`invalidates`. Its state is `Signature`, then `Body`.
  2. The body.

  A single-expression body without a return type has its type known only after phase 2.

### 3.3 Branches and loops

- **Branches**:
  - `if`: `check_cond` gives `(t, f)`. Walk the then-block from `t` and the else-part from `f`, then join the reachable ends.
  - `when`: one fork per arm. Each arm starts from the remaining state after the previous patterns' negations.
  - `and`, `or`, `??`, `?.`, `catch` fork and join the same way.
  - A `Never` branch's state goes to `FnCx.returns` / `LoopCx.breaks` / `LoopCx.continues`, not to the join.
- **Value typing of `if`/`when`/`catch`** follows values brief D-29/D-98: with an expected type, each branch is checked against it; otherwise literals of the other branches serve as hints, and the result is the normalized union of the widened types (`TY006`/`TY040`/`E-IF-2`).
- **Loop fixpoint** (only flow re-runs):

```
head = state_before
for round in 0.. {
	push flow buffer; self.flow = head.clone()
	(while: check_cond → body from true state; for: bind loop variable with the index fact) walk body
	next = widen(head, join(head, end_of_body, continues…), round)
	if next == head { keep this round's buffer (flush to parent); break }
	discard buffer; head = next;   if round == 8 { head = top_for_changed_places(head) }   // hard cap
}
after = join(false-branch of the condition at head, breaks…)   // `loop` with no break: unreachable (Never)
```

Every node of the body is visited every round. Per-node flow outputs are therefore always those of the stable round: check bits, intervals, `CAN_FAIL`, requirement checks. A failing set or a `throws` accumulator only grows across rounds, because facts only widen, so union accumulation is exact.

### 3.4 Recursion SCCs (advanced brief D2)

1. The `analysis_stack` holds frames `{ decl, low }`. Reaching a declaration whose state is `Body{frame k}` sets `low = min(low, k)` on every frame above `k`.
2. A finished frame whose `low` is lower than its index joins the root's member list. Its summary is `Provisional`: the declared effects, or empty for a single expression.
3. When the root finishes with members, it runs rounds:
   1. Push a flow buffer.
   2. Re-walk the bodies of all members. Types are memoized, so this is flow and effects only.
   3. Recompute their summaries from the accumulators.
   4. Repeat until no summary changes. Summaries only grow, so this terminates.
   5. Flush the last round's buffers, then mark the summaries `Final`.

### 3.5 Effects: inference vs checking

- **Accumulation**: each `FnCx` accumulates `throws: TypeId`, `requires`, `mutates`/`invalidates` paths (local roots dropped), module reads and the inferred bits.
- **At the end of a body**, the accumulator becomes a normalized summary:
  - **Single-expression body**: the summary is the signature's effects.
  - **Block body**: `covers(declared, actual)` gives `EFF001`/`EFF002`/`EFF004`, `E-ERR-4` for `throws`, `EF016`-class `requires` codes, and `EFF011`/`W-ERR-3` for declared effects the body does not have.
  - **Entry function, test bodies, top-level statements**: always inferred (advanced brief D51).
- **In M1 only `throws` is live**: the coverage rule of values brief §15, with failing expressions marked `CAN_FAIL`. The `mutates`/`invalidates` machinery is M2, but `FnSig` already carries an `EffectSetId` (empty in M1), so backends and signatures don't change shape.
- **A call** (advanced brief §5.7):
  1. check the arguments;
  2. substitute the callee's effects;
  3. check mutability and the alias/iteration rules;
  4. apply the effects to the flow state (drop facts, invalidate references);
  5. accumulate the non-local effects.

  Non-stored lambda arguments contribute their effects at the call.

### 3.6 Integer ranges and overflow elision

**Interval sources**: the place's fact, otherwise its type's range. Literals and folded constants are exact. Calls give their declared return range. Arithmetic uses the table of values brief §6.5 in `types/int.rs`.

At every integer arithmetic node (`+ - * // %% **`, unary `-`, compound assignment, `abs`), the flow part:

1. **Interval and compute type.** Computes the node's interval from its operands' current intervals. Picks the compute type `C`: `Int64`; `Int128` if an operand is `UInt64`/`Int128`; `UInt128` rules (values brief §6.7).
2. **Native overflow bit.** Sets `OVERFLOW_NATIVE` iff the interval is not contained in `C`. If they are disjoint: `E-NUM-10` (`report_flow`).
3. **JS overflow bit.** Sets `OVERFLOW_JS` iff the storage is number-represented and the interval is not contained in ±(2^53−1). The backend raises `E-NUM-19` itself (values brief D-9): the analyzer is target-independent.
4. **Records** `ArithInfo { interval, compute, root }`. `root` = the parent is not an arithmetic node of the same tree.
5. **Divisor and exponent proofs.** For `//`/`%%`: `DIVISOR_CHECK` iff 0 ∈ the divisor's interval set (exclusions count), `E-NUM-12` if the divisor is exactly 0, and `CAN_FAIL` with `DivisionByZero`. Exponents and shifts are handled the same way.

**At a sink** (`sink()`), for a target integer type `T`:

- the interval is contained in `T`: no flag;
- the value is an arithmetic root (values brief D-89) and its interval is not contained: `SINK_CHECK` + `coercions[n] = T`;
- the interval is disjoint from `T`: `E-NUM-10`;
- any other expression whose interval is not contained: `E-NUM-14`.

An unannotated binding's sink is the base type (values brief §6.4; `E-NUM-15` for incomparable bases). Comparisons, `/` and conversions consuming a root are not sinks.

**Backend reading**:

- **LLVM**: `OVERFLOW_NATIVE` → `*.with.overflow` at the width of `C`, otherwise `nsw`/`nuw`. `SINK_CHECK` → compare against `coercions[n]`. `ArithInfo.interval` → `range` metadata.
- **JS**: `OVERFLOW_JS` → the safe-integer check. `SINK_CHECK` → the bounds check.

Proof bits are recomputed at every visit (§3.3), so they are the loop-stable ones.

### 3.7 Narrowing

`check_cond` implements the narrowing table of values brief §14: `is`, `== none`/literal, ordering against constants and intervals, `is in` a literal array, `and`/`or`/`not`, discriminant narrowing (types brief D44).

- **A read of a narrowed place** returns the narrowed type (memoized). It records `Requirement { node, place, needs }` and sets `NARROWED`.
- **Requirement re-check**: each later visit re-checks the requirement against the current fact. A failure is `E-OPT-1` / `E-NAR-1` via `report_flow`.
- **Early exits**: after `assert c` (values brief D-27) and after an `if` whose branch is `Never`, the following code continues from the other state.

### 3.8 Generics and instantiations (M2)

- **Inference** at each call follows types brief §3.11.4. The chosen arguments go into `instantiations[call]` and may contain the `Param`s of an enclosing generic.
- **Generic bodies** are checked once against their constraints. Union constraints are checked per member (types brief D19).
- **LLVM** monomorphizes on demand from the entry: it substitutes along the chain of `instantiations`.
- **JS**: the per-member instances and dictionary needs of types brief §3.11.8 are summary bits (`FnInfo`).

## 4. Performance

- **Storage**: no `Rc`, no `Box<TypeKind>`, no strings in types. All ids are `u32`. Variable-size data lives in flat `Vec`s, interned by content through `FxHashMap`.
- **Per-node tables**: one allocation each, sized by the node count: 20 bytes per node in total. Sparse tables are used only for the minority node classes (arithmetic, calls, functions).
- **Hot paths** allocate nothing once types exist: type equality is an id compare; `assignable` and `subst` are cached; union normalization uses one reusable `SmallVec<[TypeId; 8]>` scratch buffer; facts are hash-consed; flow states stay inline in `SmallVec`.
- **Complexity**: O(nodes × (1 + loop rounds)) node visits.
  - Loop rounds are usually 2–3. Widening caps them at 8.
  - Nested loops multiply only their inner flow transfer.
  - Interning, lookups and caches are amortized O(1).
  - A join is O(|facts|) with |facts| ≈ 10.
  - **Target: under 5 ms for a 5 000-line file**, measured by a `#[ignore]` benchmark over `benchmarks/` and the documentation corpus.
- **LSP**:
  - One analysis thread owns the `Session`. Queries are messages to that thread; they take microseconds.
  - Edits are debounced (50 ms). An `AtomicBool` cancel flag is checked at every declaration boundary and loop round.
  - `update(module, text)` re-parses, re-analyzes the file, compares interface hashes per declaration, and re-analyzes only dependents that recorded a changed interface or a changed lookup (advanced brief §21.4).
  - The `TypeStore` is append-only. When it grows past 4× its size after the last rebuild, the session is rebuilt from the open files (epoch reset).
  - `node_at` is a linear scan for the smallest enclosing span: about 50 µs at 100k nodes, so no index is needed.

## 5. Milestones (3 engineers)

| Engineer | Owns |
|---|---|
| **E1, types and declarations** | `types/*`, `symbols.rs`, `syntax.rs`, `check/{mod,decl,type_expr,expr,functions,calls,objects,patterns(typing)}.rs` |
| **E2, flow and numbers** | `flow/*`, `check/{numbers,ops,control,errors}.rs`, `types/int.rs`, later `effects.rs`, `check/{effects,references}.rs`, `flow/refs.rs` |
| **E3, surface and infrastructure** | `diagnostics/*`, harness and cases, `builtins/*`, `session.rs`, `analysis.rs`, `ide.rs`, `lower/`, `check/tests.rs`, string/boolean/console handlers in `check/expr.rs`, language server wiring |

Rules that avoid conflicts:

- **`check/mod.rs` (dispatch)** and **`syntax.rs`** belong to E1. Others add one-line match arms and `View` variants in small separate PRs.
- **Codes** have one file per brief.
- **Builtins** have one file per area.
- **Interfaces are fixed in M0**, before parallel work starts: the `TypeStore` API, the `FlowState` API, and `report`/`report_flow`.

**M0. Skeleton (1 week, all in parallel)**
- **E3**:
  - crate, workspace membership, ids, interner, diagnostics and code registry;
  - `analyze_source`;
  - harness (§6);
  - an import script that turns the values brief `T-*` lines and the types brief `✓/✗` blocks into `tests/cases/**`, all marked `-- pending: M1|M2|M3`.
- **E1**: `TypeStore` with predefined ids, interning, `IntType`, display; `syntax.rs` with `View`, `children()`, literal decoding, `block_declarations`.
- **E2**: `PlaceId` trie, `FactStore`, `FlowState` fork/join/widen, `int.rs` with the full interval table and unit tests.
- **Done when**:
  - `cargo test -p fa-analyzer` passes and every imported case runs as pending;
  - the analyzer does not panic on any input of the parser's test corpus (`catch_unwind` sweep).

**M1a. Backend-ready slice (2–3 weeks). The backends start at its end.**
- **Content**:
  - names, scopes, `let`/`mutable`, assignment and compound assignment (values brief step 1);
  - scalars with literal adaptation: integers and sized integers, `Float`, `Boolean`, `String`, interpolation;
  - functions with declared and inferred return types, direct calls, recursion with `EFF020`;
  - `if`/`else` (statement and value), `while`, `for` over ranges, `loop`, `break`/`continue`/`return`;
  - comparisons and logic, arithmetic trees with intervals, every overflow/sink bit of §3.6, integer conversions;
  - `console.log`, `assert`.
- **Freeze** the backend-facing part of `Analysis`: `types`, `resolutions`, `flags`, `arith`, `coercions`, `symbols`. `analysis.rs` gets a document comment for each field.
- **Owners**: E1 declarations, functions, calls, scalar type expressions; E2 numbers, conditions, loops and fixpoint; E3 strings, booleans, console, assert, builtins, `Analysis` accessors.
- **Acceptance**:
  - values brief T-DECL-1…5 and 20…30; T-NUM-1…14, 17, 18, 21…25, 41…43, 45, 46, 49…53, 60…80, 82, 83, 86, 89, 90;
  - T-IF-*, T-STR-1…13, T-BOOL-*, T-CMP-1…11; T-LOOP-* over ranges;
  - the integer programs of `benchmarks/` and the Fibonacci example check without diagnostics;
  - golden flag tests for T-NUM-11/12/17 (no checks) and 13/14 (sink check).

**M1b. Rest of the core (4–5 weeks)**
- **E1**:
  - type declarations and aliases, object types, typed and untyped literals, defaults, field access, `readonly`, `#private`;
  - anonymous unions with normalization and `TY040`; named unions, `Errors`, enumerations;
  - optionals typing; `when` typing and patterns;
  - `Array` as a built-in `Collection` with literals, `length`, indexing, `push` on mutable places, `for … in` (read-only elements until M2).
- **E2**:
  - narrowing (§3.7);
  - `when` exhaustiveness, including integers;
  - `throws`/`try`/`catch`/`throw` and coverage (values brief §15);
  - index proofs and length facts (decisions.md "Index proofs"); divisor and conversion proofs;
  - `do` blocks and block values; unreachable code `W-DO-2`.
- **E3**:
  - `test` blocks (`TEST001/002`);
  - `lower/when.rs` for the backends;
  - language server: diagnostics, hover, definition;
  - the remaining builtins: built-in errors, `Integer.parse`, `arguments`.
- **Acceptance**:
  - values brief T-OBJ-1…5 and the rest of §11 except spreads and `.=` subtleties (M2);
  - T-OPT-*, T-NAR-* except reference cases, T-WHEN-*, T-ERR-*, T-DO-*, T-CMP-*, T-LOOP-* except E-LOOP-4 through calls, T-NUM-19, 20, 33…36, 54;
  - types brief ty-decl-*, the §3.2, §3.4, §3.5, §3.8 tests, and §4.1–4.2 without generics;
  - advanced brief §18.4 for `TEST001` and `TEST002`.

**M2. Tier 2 of the architecture scope (8–10 weeks)**
- **E1**: generics (types brief §3.11), methods and associated functions, recursive types (§3.6), type operations (§3.10), maps and sets, structural typing (advanced brief §10).
- **E2**: effects II (advanced brief §3, types brief §4.3), the reference checker (advanced brief §4–5), closures and captures, requirements (§9), copies and moves bits (§6).
- **E3**: multi-file modules (§15), `ModuleResolver` over `fa-codebase-parser`, incremental `update` and interface hashes, completions, tuples, ranges, destructuring, the remaining number kinds (values brief step 10), the prelude moved to `packages/std`.
- **Acceptance**:
  - advanced brief §3.8, §4.5, §5.13, §9.9, §10.8, §15.9;
  - types brief §3.6, §3.10, §3.11.10, §4.3.6–§4.8, §5.1–§5.4;
  - values brief T-TUP-*, T-RNG-*, T-DST-*, T-DECL-6…13 and 35…37, and the rest of T-NUM-*.

**M3. Tier 3** (advanced brief tiering): pools, columnars and growing collections, relations, compile-time evaluation (`static`, `@`), reactivity, foreign code, `for parallel`, configuration files. The owners follow the same areas: E1 types and collections, E2 flow, effects, relations and reactivity, E3 the interpreter, configuration and packages.

## 6. Testing

- **Unit tests** are colocated in each module:
  - `TypeStore`: structurally equal ⇒ same id; flags.
  - `union.rs`: every rule of §3.4.1, `TY040` matrix.
  - `int.rs`: the §6.5 table, compute types, exclusions.
  - `relate.rs`: assignability matrix, `Integer`/`Int64`, `Float32`/`Float`, `Character`/`String`.
  - `flow/`: join, widen, `drop_below`.
  - `codes`: uniqueness.
  - `syntax.rs`: views over parser outputs, `children()` completeness.
- **The data-driven harness** is `tests/cases.rs`. It walks `tests/cases/**/*.fa`. A directory containing `main.fa` is one multi-file program (M2). For each file, `analyze_source` runs, then:
  - **Every expected diagnostic must be emitted on its line with its code or an alias. Every emitted error or warning must be expected.** Hints are ignored.
  - **The `AN0xx` codes** always fail a case unless the case expects them.
- **Annotations** are Fa comments:

```fa
let e: Int8 = 100 + 100          -- error: E-NUM-10
let s = a + b                    -- error: E-NUM-15, warning: W-NUM-2      (several on a line)
let x = compute()
-- ^ error: E-ERR-3                                                        (applies to the previous code line)
let wide: Int16 = a + b          -- no-check                              (no OVERFLOW_*/SINK_CHECK bit on this line)
let avg: Int8 = f(a)             -- check: SINK_CHECK
let count = 0                    -- type count: Integer                   (hover type of the first `count` on the line)
console.log(x)                   -- prints: 3                             (ignored here; used by backend e2e tests)
```

- **Header lines**:
  - `-- pending: M2 (reason)`: an expected failure; the case is reported "now passes, remove pending" when it starts passing.
  - `-- kind: standalone | module | test`: default `standalone`.
  - `-- target: js`: backend-only expectations.
- **Formats**: the colon is optional (`-- error REF001` from the advanced brief is accepted). Codes in a list are separated by commas.
- **Imported cases** keep their brief id in the file name (`T-NUM-70.fa`, `ty-decl-4.fa`, `adv-5.13-07.fa`), so a failure points back at the brief.
  - The import script maps codes through `tests/code-map.txt`. Exact duplicates are already aliases. Non-identical overlaps (for example `TY002` where the values brief says `E-NUM-14`) are settled once there, by a reviewer.
- **Corpus tests**:
  - every ```fa block of `documentation/` must analyze without panicking;
  - complete programs must check clean (`#[ignore]` until M1b, then on);
  - the end-to-end directory of the architecture doc reuses the same `.fa` files with their `prints`/`exit` lines.

## 7. Dependency on the parser, kept robust

- **Only `syntax.rs` names `fa_parser::Node` variants.** It ends with `_ => View::Unsupported(kind_name)`, so a new parser node never breaks the analyzer's build: it shows up as `AN001` until it is mapped. A renamed or reshaped variant breaks one file.
- **`View` is the analyzer's own enum.** It also normalizes parser quirks:
  - n-ary `Add { operands }` folded left;
  - `Power` folded right;
  - comparison chains;
  - `Integer(i64)` today, arbitrary-precision text later (values brief D-13), both decoded to `I256`;
  - `Node::Error` (the `Error` type keyword) vs future error nodes.
- **The analyzer assumes**:
  - the tree is immutable after `parse` (`TypedSyntaxTree::remove` must not be called after parsing);
  - `nodes[i]` and `spans[i]` are aligned;
  - children are referenced by index, with no order assumption.

  It never reads the nodes' `resolved_type` fields or the parser's `hoisted_symbols` (A5).
- **Requests to the parser agent**, none blocking for M0:
  1. `Node::children(&self) -> impl Iterator<Item = usize>` and `Node::kind_name()` next to the enum. Until they exist, `syntax.rs` has its own copy.
  2. Error nodes with spans instead of panics, and `Access` with a missing member for `x.` (needed for completions).
  3. Remove the stub `src/analysis/`, `types.rs`, `type_error.rs`, `scope.rs`, the `resolved_type` fields, and the `analyze()` call in `parse()` (it prints "Analyzing…").
  4. Parse diagnostics as plain data (no `tower-lsp` dependency). The analyzer converts them into its own `Diagnostic` at the boundary.
  5. Owned source text (`Arc<str>`) instead of leaked `&'static str` input. The language server re-parses on every edit, and each leak accumulates.
  6. The precedence fixes D-1, D-2, D-3 and literal fixes D-11…D-16 of the values brief §4. The view layer can absorb the tree shape, but not wrong precedence.

### Critical Files for Implementation
- /home/claude-dev/projects/Fa/design/compiler-architecture.md
- /home/claude-dev/projects/Fa/design/implementation/types-functions-collections.md (§2 type representation, §3.4 unions, §3.11 generics, §4.3 effects, dependency order)
- /home/claude-dev/projects/Fa/design/implementation/values-and-control-flow.md (§2, §6.4–6.7 ranges/overflow, §14 narrowing, §15 errors, dependency order)
- /home/claude-dev/projects/Fa/design/implementation/advanced-and-architecture.md (§2 single walk, §3–5 effects and reference checker, §15 modules, §21 incrementality)
- /home/claude-dev/projects/Fa/packages/parser/src/nodes.rs and /home/claude-dev/projects/Fa/packages/parser/src/typed_syntax_tree.rs (the `Node` enum the view layer adapts)

## 8. How to add a handler

Every `View` variant already has an arm in `check/mod.rs` that calls a stub `check_*` method in its area's file (`check/decl.rs`, `expr.rs`, `numbers.rs`, …). The stub returns `self.unsupported(n, "…")`, which reports `AN001` once and walks the children quietly. To implement a construct, replace the stub's body in its own file; `check/mod.rs` doesn't change. A new parser node needs a `View` variant in `syntax.rs` (with a unit test) and one arm.

```rust
pub(super) fn check_not(&mut self, n: NodeId, value: NodeId) -> TypeId {
	let actual = self.check(value, Some(TypeId::BOOLEAN)); // children: every visit (flow)
	self.sink(value, actual, TypeId::BOOLEAN);             // flow part: reports with report_flow
	self.memo(n, |w| {                                     // type part: runs once per node
		// type rule; type diagnostics: w.report(w.diagnostic(&codes::…, n, "message"))
		TypeId::BOOLEAN
	})
}
```

- **Type part** (`memo`): resolutions (`set_resolution`), declarations (`declare`), type diagnostics with `report`. It must return a set type (`VOID` for a statement).
- **Flow part** (outside `memo`): facts, flags (`set_flag` / `clear_flag`, recomputed at each visit), `sink`, and `report_flow`, which loop and SCC rounds buffer (A3).
- **Statement position**: `self.is_statement(n)` (an `if` there is `Void`). Places: `check_place`. Types written in annotations: `check_type_expr`, never `check`.
- **Tests**: a unit test in `check/walk_tests.rs` for the side tables, and a case in `tests/cases/` with `-- error: CODE` annotations (§6). Remove the `-- pending` line of every imported case that now passes (the harness lists them).
