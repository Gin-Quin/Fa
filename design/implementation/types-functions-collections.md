# Implementation brief: types, functions and collections

Working brief for the engineers of `fa-checker` (shared by every target), the JavaScript backend and `fa-native` / `fa-llvm`. It covers `documentation/types/*`, `documentation/functions/*` and `documentation/collections/*`, cross-checked with `documentation/reference/language-reference.md`, `advanced/structural-typing.md`, `advanced/reference-checking.md`, `values/*` (numbers, errors, objects, tuples), `control-flow/*` (narrowing, `when`, loops) and the design records `design/compilation-with-llvm.md` and `design/spec-review.md`. A sibling brief covers the other areas; where this one needs a rule from them (narrowing, range analysis, reference checking), it states the interface it relies on and links the page.

The documentation stays the source of truth. Where it is silent or contradictory, this brief **decides** (section [Ambiguities and decisions](#ambiguities-and-decisions), items `D1`…): the project lead accepts or overrides. Every rule below that depends on a decision cites it.

---

## Contents

0. [Conventions](#0-conventions)
1. [Where this sits in the checker](#1-where-this-sits-in-the-checker)
2. [Internal type representation](#2-internal-type-representation)
3. [Types](#3-types)
4. [Functions](#4-functions)
5. [Collections](#5-collections)
6. [Diagnostics index](#diagnostics-index)
7. [Ambiguities and decisions](#ambiguities-and-decisions)
8. [Dependency order](#dependency-order)

---

## 0. Conventions

- `S ≤ T`: a value of type `S` **satisfies** `T` when read (the docs' "satisfies"; covariant, structural). `S ≡ T`: same interned `TypeId`. `S ⊑w T on P`: `S` can be **written through** a `T` on the paths `P` (invariant on the written leaves, see §3.2.4).
- "Place", "path", "flat", "owning edge", "may designate", "fresh": as defined in `advanced/reference-checking.md`.
- "Facts": the flow facts (exact literal, integer interval plus excluded values, narrowed member, collection length) kept per place by the flow walk (`values/numbers#range-analysis`, `control-flow/conditionals#narrowing`). Facts are **never** part of a `TypeId`.
- Diagnostics have stable codes: `TYnnn` types, `GNnnn` generics, `TFnnn` type functions and reflection, `CVnnn` conversions, `FNnnn` functions, `EFnnn` effects, `CLnnn` collections. `W…` are warnings. The message texts quoted from the docs are normative wording; the others are suggestions. Every diagnostic has a primary span and, where useful, notes pointing at the declaration (e.g. the generic declaration a type-function error comes from).
- Tests in one fenced block may reuse the declarations of the previous tests of the same block, unless they redeclare them.
- Test cases: `✓ id` is a valid program with its expected standard output (one line per `console.log`, Fa display format: strings raw at top level, `{ name = "John" }`, `[1, 2]`, `Color.red`…); `✗ id` is an invalid program with the expected diagnostic code (and the line it points at when not obvious). Tests are written as `*.test.fa`-ready snippets; a top-level standalone file is assumed (statements allowed).
- Parser node names (`Node::…`) are those of `packages/parser/src/nodes.rs` today. "As parsed" lines show the parser's `to_string()` rendering, checked with the current parser.

---

## 1. Where this sits in the checker

`fa-checker` is a set of **memoized queries with cycle detection** (`compilation-with-llvm.md#architecture`). The ones this brief defines:

| Query | Result | Notes |
| --- | --- | --- |
| `type_decl(DeclId)` | `TypeDeclInfo`: kind (alias, object, `Enum`, `Fields`, `Union`, `Errors`, `CStruct`, type function), generic parameter list, members in declaration order, methods, associated functions, nested types, field defaults (as unevaluated expressions) | Lazy. Recursive SCCs are built together (§3.6). |
| `apply_type(DeclId, args: ListId)` | `TypeId` | Memoized on `(DeclId, args)`. Carries a nesting counter: more than 256 nested applications is `TF001`. |
| `resolve_type_expr(NodeId, env)` | `SpelledType { id: TypeId, spelling: SpellingId }` | Interprets an expression node in **type mode** (§3.1.1). |
| `signature(FnId)` | `FnSig` + `FnDeclInfo` (names, defaults, generic parameters, chain) | For a single-expression body, depends on `infer_body`. |
| `infer_body(FnId)` | `BodySummary` | Return type, `throws`, `requires`, `mutates`, `invalidates`, purity, escape bit of each function-typed parameter, freshness of a returned reference, write-set per parameter (§3.2.4), "needs full type" per parameter (`Type(parameter)`), JavaScript dictionary needs per type parameter (§3.11.8). |
| `check_body(FnId)` | diagnostics + typed tree | Block bodies: compares the summary with the declared signature. |
| `assignable(S, T, mode)` | `bool` + reason | Cached on `(S, T, mode)`, coinductive for recursive types. |

**One walk per body** computes types, facts (range analysis, narrowing) and the `throws` set together, because `throws IndexError` / `KeyNotFound` / `DivisionByZero` / `ArithmeticError` depend on what the facts prove. Reference checking (`advanced/reference-checking.md`) is a second walk on the typed tree, which only needs the callee summaries.

**Recursive functions.** `signature(f)` requested while `infer_body(f)` is running is a cycle. If `f` declares its return type, the cycle is an SCC: the SCC's effect summaries start empty and are iterated to a fixpoint (all effect sets are finite and grow monotonically). If `f` (or any member of the SCC that is reached before its body is inferred) has no declared return type: `FN010` "a function that calls itself must declare its return type".

---

## 2. Internal type representation

Goals: O(1) type equality, O(1) hashing, no allocation per type operation once a type exists, cache-friendly, deterministic stable hashes for the incremental cache, shareable across threads.

### 2.1 Handles

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash)] pub struct TypeId(NonZeroU32);   // index in TypeTable
#[derive(Copy, Clone, Eq, PartialEq, Hash)] pub struct Sym(u32);              // interned identifier or string literal
#[derive(Copy, Clone, Eq, PartialEq, Hash)] pub struct ListId(u32);           // interned [TypeId] (start, len) in one Vec<TypeId>
#[derive(Copy, Clone, Eq, PartialEq, Hash)] pub struct ConstId(u32);          // interned compile-time value
// and ShapeId, FieldsShapeId, SigId, PathId, EffectSetId, NominalId, IntId, MaskId, ParamId, RecId: all u32 into interned side tables
```

Every side table is **hash-consed**: building the same thing twice returns the same id. The global interner is sharded by hash (N shards, each a `Mutex<FxHashMap<Kind, Id>>` + append-only `Vec`), so parallel checking of modules shares types. Ids are only stable within a session; every interned item also caches a **stable 128-bit structural hash** (computed from names and children's stable hashes, never from ids) used for the incremental cache, for the canonical member order of anonymous unions on native (`compilation-with-llvm.md#unions-and-optionals`) and for requirement keys.

### 2.2 `TypeKind`

`TypeKind` is `Copy`, at most 16 bytes (a `u8` tag plus up to 12 bytes of payload). Variable-size data lives in the side tables.

```rust
pub enum TypeKind {
	// ── special ───────────────────────────────────────────────
	Never, Void, None,
	ErrorTop,                 // the built-in `Error`: any error
	Unknown,                  // error recovery: satisfies and accepts everything, never reported twice
	// ── scalars ───────────────────────────────────────────────
	Boolean, BoolLit(bool),
	Int(IntId),               // IntType { min: I256, max: I256, storage: IntStorage }; a literal is min == max
	SymInt(SymIntId),         // inside a generic declaration only: bounds affine in compile-time value params (`rows - 1`)
	Float(FloatBits),         // F32 | F64
	FloatLit(FloatBits, ConstId),
	BigInt, BigIntLit(ConstId),
	Percentage,
	String, StringLit(Sym), Character,
	// ── structured ────────────────────────────────────────────
	Object(ShapeId),          // Shape { fields: &[FieldEntry] sorted by name bytes, nominal: Option<DeclId> }
	Tuple(ListId),
	Union(ListId),            // anonymous, normalized (§3.4.1), len >= 2, sorted by TypeId
	NamedUnion(NominalId, MaskId),  // a `Union`/`Errors` type, or a subset of its members (difference, narrowing)
	Member(NominalId, u16),   // `Shape.Circle`
	Enum(NominalId, MaskId),  // an enumeration, or a subset of its members
	EnumMember(NominalId, u16),
	Fields(FieldsShapeId),    // structural flag tree, declaration order (D14)
	CStruct(NominalId),
	Function(SigId),
	Collection(CollKind, TypeId /*item*/, TypeId /*key or NONE*/),
	Iterator(TypeId /*item*/, PathId /*collection place*/),
	Ref(TypeId /*target*/, PathId /*owner*/),   // `T in path`
	Range, Task(TypeId, TypeId /*throws*/),
	// ── generic machinery ─────────────────────────────────────
	Param(ParamId),           // a type parameter in a generic body (constraint in ParamInfo)
	ImplicitParam(FnId, u16), // `Type(parameter)` of a structural parameter (§3.11.7)
	Const(ConstId),           // a compile-time value given to a lowercase parameter, as a type argument
	Rec(RecId),               // node of a canonical recursive graph (§3.6)
}
```

Notes:

- **Predefined ids.** `NEVER`, `VOID`, `NONE`, `ERROR`, `UNKNOWN`, `BOOLEAN`, `TRUE`, `FALSE`, `INTEGER`, every sized integer, `FLOAT`, `FLOAT32`, `BIG_INT`, `PERCENTAGE`, `STRING`, `CHARACTER`, `RANGE` are interned first, at fixed indices: hot checks are integer compares.
- **Integers** (`D2`). `IntStorage = Default | I8 | I16 | I32 | I64 | I128 | U8 | U16 | U32 | U64 | U128`. `Integer` is `Int { min = -2^63, max = 2^63-1, storage = Default }`; `Int64` is the same range with `storage = I64`; `Integer(min = 0)` is `Default` storage with `min = 0`; `Byte` is the `U8` type itself (an alias). Bounds are 256-bit (`I256`, two `i128`), so interval arithmetic on 128-bit types never overflows. An integer literal type is `min == max`. The storage decides the native width and the JavaScript class (`number` for `Default`, `I8`–`I32`, `U8`–`U32`; `bigint` for `I64`, `U64`, `I128`, `U128`).
- **Shapes.** `FieldEntry { name: Sym, ty: TypeId, flags: u8 }` with `READONLY`, `REACTIVE`, `RELATION` (a direct or reverse relation side). Only **public data fields** enter a shape (function-pointer fields are data fields). Methods, associated functions, defaults and nested type declarations live in `TypeDeclInfo`, keyed by the **declaration used at the use site** (`SpellingId`), never by the shape (`advanced/structural-typing.md#what-is-compared`). A type with `#private` fields has `nominal = Some(decl)`; its private fields are kept in the shape too (layout) but are not part of structural comparison against other types.
- **Spelling.** Identity ignores names. Every type expression node stores `SpelledType { id, spelling }`, where `SpellingId` points at the name and arguments as written (`UserId`, `Point(Integer)`). Diagnostics, `@typeToString`, `@typeToSchema`, method resolution and field declaration order use the spelling. A value's `Type(value)` carries the spelling it was declared or created with.
- **Named unions and enumerations** are nominal: `NominalInfo { decl: DeclId, args: ListId, kind: Union | Errors | Enum | CStruct | Private }`. `NamedUnion(n, ALL)` is the declared type; `NamedUnion(n, mask)` is `Shape - Shape.Point` or a narrowed subset (`D10`); a mask of one member normalizes to `Member(n, i)`. An anonymous union whose members are all members of the same named union normalizes into the masked form. `MaskId` interns bitsets (a `u64` inline for up to 64 members, else a side table).
- **Function signatures.** `FnSig { params: &[ParamSig], ret: TypeId, throws: TypeId, requires: &[(Sym, TypeId)], effects: EffectSetId, is_async: bool, passthrough: u32 }`, `ParamSig { ty: TypeId, variadic: bool, stored: bool }`. Parameter **names are not part of the identity** (they are kept in the spelling, for `Parameters` and messages); effect paths refer to parameters by index. `throws` is `NEVER` for no errors. `requires` is sorted by the stable hash of the requirement type (`advanced/requirements.md` keys requirements by shape). `passthrough` is the bitmask of function-typed parameters that are **not stored** (their effects become effects of the call, §4.5). Named arguments, defaults, generic parameters and composition chains are not types: they live in `FnDeclInfo`.
- **Collections.** `CollKind = Array | GrowingArray | Set | GrowingSet | Map | GrowingMap | Pool | GrowingPool | Columnar | GrowingColumnar | OrderedColumnar | RelationPool(RelationId)`. `key` is `NONE` except for maps.
- **Reference types** `Ref(T, owner)` appear in parameter types, return types, relation fields and inferred `use` types. Locals never hold `Ref` inside another type except `Optional` (`(T in c)?`) and `Iterator` (`D23`).
- **Compile-time values.** `ConstValue = Bool | Int(I256) | String(Sym) | Character(Sym) | EnumMember(NominalId, u16) | FieldsValue(FieldsShapeId, Bitset) | KeySet(FieldsShapeId, Bitset) | Array(ListOfConst) | Tuple(..) | Object(ShapeId, ..)`, hash-consed: equal values have equal `ConstId`, so `Matrix(Float, 2 + 2, 4) ≡ Matrix(Float, 4, 4)` falls out of interning the argument list.

### 2.3 Cached per-type flags

Computed once when a `TypeId` is interned, stored in a parallel `Vec<TypeFlags>` (`u32`):

| Flag | Meaning | Used by |
| --- | --- | --- |
| `HAS_PARAM` | contains `Param`/`ImplicitParam`/`SymInt` | substitution fast path |
| `FLAT` | invalidation-free (`reference-checking#owning-edges`): primitives, literal unions, enums, fields, relation references, objects of flat fields | reference checker |
| `TRIVIALLY_COPYABLE` | owns no memory | native copy/drop, JS copy elision |
| `COMPARABLE` | `==` defined: no function, no `Task`, no thread-safe handle field (`D17`) | `==`, `Set`, `Map` keys, `includes` |
| `ORDERED` | `<` defined: numbers of one kind, `Percentage`, `String`, `Character` | `sort()`, comparisons |
| `MOVE_ONLY` | contains `Task` or a type with `destroy` | copies, escaping captures, collections |
| `HAS_VOID` | contains `Void` in a data position | `TY071` |
| `RECURSIVE` | part of a recursive graph | `DeepKeys`, heap boxing |
| `KIND` (4 bits) | runtime kind for union discrimination (§3.4.2) | `TY040` |
| `JS_BIGINT` | represented as a JavaScript `bigint` | JS lowering, views (`D2`) |

### 2.4 Effects and paths

```rust
pub struct Path { root: Root, steps: ListOf<Step> }          // interned → PathId
pub enum Root { Param(u16), SelfParam, Module(DeclId), Requirement(u16), Local(LocalId) }
pub enum Step { Field(Sym), Private(Sym), Index(IndexTerm), Range(Option<IndexTerm>, Option<IndexTerm>, bool), All, Length, Size }
pub struct IndexTerm { base: Term, offset: i64 }
pub enum Term { Lit(ConstId), Param(u16), Local(LocalId), Length(PathId), Size(PathId) }
pub struct EffectSet { mutates: SmallVec<[PathId; 2]>, invalidates: SmallVec<[PathId; 2]> } // interned → EffectSetId
```

An `EffectSet` is normalized at interning: a path covered by another of the same or a stronger kind is dropped (`mutates p` covers `mutates p.x`, `[..]` covers every index and key, `invalidates p` covers `mutates p`), and the implied `mutates c.size` of `invalidates element` with `element: T in c` is added. Signatures store `Param(i)` roots; a call substitutes argument places for them. `Local` roots only exist during a body walk and are dropped from summaries.

### 2.5 Facts (not types)

```rust
pub struct Fact { literal: Option<ConstId>, int: Option<Interval /* I256 bounds */>, excluded: SmallVec<[I256; 2]>,
                  narrowed: Option<TypeId>, length: Option<Interval>, key_present: SmallVec<[ConstId; 2]> }
```

The flow walk keeps `FxHashMap<PlaceId, Fact>` per program point (persistent maps, or a scoped undo log, so branches are cheap). Joins: interval hull, union of `narrowed`, intersection of `excluded` and `key_present`, literal kept only if equal.

### 2.6 Assignability cache

`assignable(S, T, Mode)` with `Mode = Read | Copy | Write(PathSetId) | Exact`, memoized in a sharded `FxHashMap<(TypeId, TypeId, Mode), Result>`. Recursive types use the classic coinductive algorithm: a pair under evaluation is assumed true; a failure invalidates only the entries computed under that assumption (keep a stack depth tag). Because recursive types are canonicalized (§3.6), `S ≡ T` short-circuits most calls.

### 2.7 What is allocated

- Interning a type allocates only when it is new (amortized `Vec` push).
- Substitution (`subst(TypeId, &[TypeId]) -> TypeId`) short-circuits on `!HAS_PARAM`, and memoizes `(TypeId, ListId)`.
- Union normalization uses a reusable `SmallVec<[TypeId; 8]>` scratch buffer per thread.
- No `Rc`, no `Box<TypeKind>`, no strings in `TypeKind`.

---

## 3. Types

### 3.1 Type declarations, aliases, literal types, inferred types

#### 3.1.1 Syntax as parsed

- `type Name(Params) = Expr` → `Node::Type { name, parameters, expression }`. `mutable type` is a parse error today; the checker still owns `TY001` for tools that build trees.
- There is **no separate type grammar**: a type is an ordinary expression node, interpreted in *type mode* by `resolve_type_expr`. The checker decides the mode from the position (annotation after `:`, right side of `type`, generic argument of a type, `is`/`when` pattern, `throws`, `requires`, `in`).

| Node (type mode) | Meaning |
| --- | --- |
| `Identifier` | a type name, a type parameter, a built-in type, a `static` value (its literal type), a lowercase compile-time parameter (only inside bounds) |
| `Access [A, B]` | `Shape.Circle`, `Color.red`, a type in a namespace |
| `FunctionCall { function, parameters }` | generic application `Array(Integer)`, `Pair(String, Float)`, `Integer(min = 0, max = 10)` (named arguments: refinement), `Type(value)`, built-in type functions |
| `MemberIndex { target, members }` | `Enum { … }`, `Fields { … }`, `Union { … }`, `Errors { … }`, `CStruct { … }`; in expression mode, a typed literal `Point(Float) { x = 1 }` |
| `Members { items }` | object type; items: `Assignment { name, type_expression, expression }` (`x: T = d`), `Readonly`, `Spread`, `Function` (method / associated function), nested `Type`; in a key-set position, a key set |
| `Union { operands }`, `Subtract`, `Optional`, `Group` | `|`, difference, `?`, parentheses |
| `ArrowFunction` (no body other than a type) | function type; `Throws`, `Requires`, `Effect` wrap the signature |
| literals, `None` | literal types, `None` |
| `When`, `If`, `Do` | type-function logic (§3.12.3) |
| `In`, `Relation`, `InRelation` | `T in path`, `people->bestFriend`, `in people->friends` |

Parser gaps relevant here (found while writing this brief): `Integer??` tokenizes as the `??` operator; `x is A | B` parses as `(x is A) | B` instead of `x is (A | B)`; `async (…) => T` function types and `function f: T = …` are rejected; `mutates c[i].d` and `mutates players[..].score` are rejected (see [dependency order](#dependency-order), step 0).

#### 3.1.2 Static semantics

- A type declaration binds a name in the type namespace of its scope (module, namespace, type body, block). Types and values share one namespace per scope for collision purposes (`TY009` "`X` is already declared"), since `Point { … }` and `Point.create` must resolve without ambiguity.
- **Alias.** `type A = T` (where `T` is not an object type literal, constructor or type function body with parameters) makes `A` the *same* `TypeId` as `T`, with spelling `A`. Every type declaration is an alias of what its right side produces, except the nominal constructors (`Union`, `Errors`, `Enum`, `CStruct`) and object types with `#private` fields, which create a new nominal identity per declaration (per application for generic ones).
- **Literal types.** String, integer (a leading `-` belongs to the literal), float, boolean literals, and `none` are types. A `static` value used as a type is its literal type (`TF` page: `type LiteralAnswer = answer`). Floats are allowed as literal types (`Schema.FloatLiteral` exists) but not as compile-time parameters (`GN006`).
- **Inferred type of an unannotated binding** (T1): the *widened* type of the value; the exact literal, the range, the narrowed member are facts. Widening table:

| Value | Type | Fact |
| --- | --- | --- |
| integer literal / integer arithmetic | `Integer`, or the base type of the operands (`values/numbers#ranges-through-arithmetic`) | exact value / interval |
| float literal | `Float` | literal |
| string / boolean literal | `String` / `Boolean` | literal |
| `Color.red` | `Color` | member `red` |
| `Shape.Circle { … }`, `UserId.User(12)` | the whole named union (`D53`) | member `Circle` |
| object literal without type | anonymous object of the widened field types | per-field facts |
| list literal without context | `Array(J)`, `J` the join of the widened element types (`D52`) | length |
| tuple literal | `Tuple` of the widened element types | |
| lambda | its function type | |
| `if`/`when`/`loop` without expected type | the join of the widened branch types | |

- **Join** (no expected type): normalize the union of the widened types (§3.4.1). Integer and float *literals* mixed in one join: `TY006` "write `1.0`, or annotate". The join must be distinguishable (`TY040`).
- **Annotated binding**: the annotated type; the value is checked against it (literals adapt).
- **Creating values.** Object values only with braces: `T { … }` without context, `{ … }` with context. `Point(12, 13)` or `Point(x = 12, y = 13)` on an object type: `TY005` "values are always created with braces: `Point { x = 12, y = 13 }`". The one exception: a named-union member with a non-object payload, `OrderId.Order(12)` (§3.5).
- Writing the type twice (`let p: Point = Point { … }`, `let s: Set(Integer) = Set.from(…)` without options): `WTY001`, formatter rewrites.
- A generic type in an annotation without its non-defaulted parameters: `TY003` "`Point` needs its parameter `NumberType`".
- Capitalization has meaning only in generic parameter lists (§3.11.1). `type main = { … }` is valid.

#### 3.1.3 Diagnostics

| Code | Condition |
| --- | --- |
| `TY001` | `mutable type` |
| `TY002` | a value doesn't satisfy the expected type (`"goodbye"` doesn't satisfy `Greeting`) |
| `TY003` | generic type used without its required parameters |
| `TY005` | positional or named call used to build an object value |
| `TY006` | integer and float literals mixed in a join without expected type |
| `TY007` | unknown type name (suggest close names and the value namespace) |
| `TY008` | a value used where a type is expected (a non-`static` value, a function) or a type used as a value (`let x = Integer`) |
| `TY009` | duplicate declaration in a scope |
| `WTY001` | type written twice |

#### 3.1.4 Runtime, JavaScript, native

Types are erased; nothing is emitted for a type declaration. A `static` literal used as a type emits nothing. Facts only remove checks.

#### 3.1.5 Tests

```fa
-- ✓ ty-decl-1: literal types and aliases → "hello" / 12
type Greeting = "hello"
type UserId = Integer
let greeting: Greeting = "hello"
let userId: UserId = 12
let count: Integer = userId
console.log(greeting)
console.log(count)

-- ✗ ty-decl-2: TY002 on the second line
type Greeting = "hello"
let greeting: Greeting = "goodbye"

-- ✓ ty-decl-3: facts make an assignment valid → 5
let five = 5
let small: Integer(min = 0, max = 10) = five
console.log(small)

-- ✗ ty-decl-4: TY002, the fact is dropped by the mutation
mutable count = 5
count += readNumber()
let other: Integer(min = 0, max = 10) = count

-- ✓ ty-decl-5: join of branches → "yes"
let ready = true
let label = if ready { "yes" } else { 0 }
console.log(label)

-- ✗ ty-decl-6: TY006
let ratio = if ready { 1 } else { 0.5 }

-- ✓ ty-decl-7: annotated join → 1.0
let ratio: Float = if true { 1 } else { 0.5 }
console.log(ratio)

-- ✗ ty-decl-8: TY003
type Point(NumberType) = { x: NumberType, y: NumberType }
let other: Point = { x = 12, y = 13 }

-- ✗ ty-decl-9: TY005 (both lines)
type Point = { x: Integer, y: Integer }
let a = Point(12, 13)
let b = Point(x = 12, y = 13)

-- ✓ ty-decl-10: WTY001 only (warning), prints { x = 0, y = 0 }
type Point = { x: Integer, y: Integer }
let origin: Point = Point { x = 0, y = 0 }
console.log(origin)

-- ✓ ty-decl-11: static value as a literal type → 42
static answer = 42
type LiteralAnswer = answer
let a: LiteralAnswer = 42
console.log(a)

-- ✗ ty-decl-12: TY002 (43 is not the literal type 42)
static answer = 42
type LiteralAnswer = answer
let a: LiteralAnswer = 43

-- ✗ ty-decl-13: TY008
let x = 3
type T = x

-- ✓ ty-decl-14: lowercase type name is legal → { foo = "a" }
type main = { foo: String }
console.log(main { foo = "a" })

-- ✗ ty-decl-15: TY007
let x: Strnig = "a"
```

### 3.2 Object types

#### 3.2.1 Syntax as parsed

`type T = { … }` → `Members { items }` with items: `Assignment { name, type_expression: Some(T), expression: Option(default) }` for a field, `Assignment { type_expression: None, expression: Some(d) }` for a field typed by its default, `Readonly { right }`, `Spread { right }`, `Function { … }` (method if the first parameter is `self`, associated function otherwise), nested `Type` declarations (relations page: `type Town = { people: Pool(Person) = [], type Person = { … } }`). Private names are identifiers starting with `#`. A typed object literal is `MemberIndex { target: type expression, members }`; an untyped one is `Members`.

#### 3.2.2 Static semantics

**Members.**

| Member | Stored | In shape | Rules |
| --- | --- | --- | --- |
| `x: T`, `x: T = d`, `x = d` | yes | yes (public) | `d` evaluated at each creation that omits `x`; it can't reference other fields or `self` (`TY013`); no annotation → widened type of `d` |
| `readonly x: T` | yes | yes, flag `READONLY` | writable only through `self` paths inside the type's own methods (`TY022`); can be given in a literal |
| `reactive x: T` | yes | yes, flag `REACTIVE` | see `advanced/reactivity` |
| `#x: T = d` | yes | private part | readable/writable only inside the type's functions (`TY016`); a literal outside the type can't give it; a type whose private fields have no default can only be built by its own functions (`TY016`) |
| `f: (…) => R` | yes | yes | function pointer: a data field; makes the type not `COMPARABLE` |
| `function m = (self, …) …` | no | no | method |
| `function g = (…) …` | no | no | associated function, called `T.g(…)`, inside the type `Self.g(…)` |
| `type N = …` | no | no | nested type, `T.N` outside (and its fields may name the enclosing type's collections, `reference-checking#reference-types`) |

- Duplicate field name `TY010`; a field and a method with the same name `TY011`; duplicate method `TY012`.
- `self` is only legal as the first parameter of a function declared inside a type, without annotation (`FN040`, `FN041`). `Self` is the type being declared (with its own generic parameters applied).
- An optional field (type accepts `None`) without default defaults to `none`.

**Identity** (T5): shape = public data fields `(name, type, READONLY, REACTIVE, RELATION)`. Two declarations with equal shapes are one `TypeId`; methods and defaults are resolved through the spelling at the use site. With `#private` fields the identity is nominal (`D3` for how a nominal type still satisfies structural types).

**Object literals** (expected type `T` an object type or a member with an object payload):

1. Each field written must exist in `T`: otherwise `TY014` "`z` is not a field of `Point`".
2. Each field of `T` without default and whose type doesn't accept `None` must be written: `TY015` "missing field `y`".
3. A field written twice: `TY017`.
4. `{ id }` is `{ id = id }`; `id` must resolve to a value.
5. `{ ...v, f = e }`: copies the public data fields of the existing value `v`; `v` must not have a field `T` lacks (`TY014`, `D9`); a field written after the spread replaces the copied one and must have the type of the field it replaces (`TY018`); a field written before the spread and copied again is `TY017`.
6. Each value is checked against the field type with that type as expected type (literals adapt, nested literals take their type from the field, lambdas take their function type from function-pointer fields).
7. Where `T` is an anonymous union, the literal must have exactly the fields of one member (`TY041` when it fits several, `TY014`/`TY015` against the closest member otherwise). Where `T` is a type parameter being inferred, the literal gives it its full type.

**Field access** `e.f`: `f` must be a field of the static type of `e` (`TY019` "`powers` is not part of `Human`"); `#f` outside the type `TY016`; on an optional, `TY020` "use `?.`"; on an anonymous union, allowed when every member has `f` (`D6`), with the union of the field types; on a named-union value, only after narrowing to a member (`TY021` "narrow `shape` to a member first"). A method call `e.m(…)` resolves `m` in the declaration of the static spelling of `e`'s type (or, in generic code, of the actual type, §3.11.6).

**Writes.** `e.f = v`, `e.f op= v`, `e .= { … }`: the root place must be mutable (`TY023` "`x` is immutable"; for a parameter, `EF001` "add `mutates p`"); `f` must not be `readonly` outside the type (`TY022`) and must not be a method (`TY024` "`describe` is a method and cannot be replaced"); `.=` only names existing fields (`TY014`), each with a value of its type, and is exactly a list of assignments (shallow).

**`==` / `!=`**: defined when the static type is `COMPARABLE` (`TY025` "values containing functions can't be compared"); compares the public data fields **of the static type** (relation and private fields are not compared).

#### 3.2.3 Assignability (`S ≤ T`, objects)

- `T` structural object: `S` is an object (structural or nominal), a member with an object payload (`D4`) or a type parameter whose constraint is such, and for every field `f` of `T`: `S` has a public field `f` with `S.f ≤ T.f`. Missing field, even optional: no (`TY026` "a `Human` has no `email` field").
- `T` nominal (`#private`): `S ≡ T`.
- Copy (`let h: Human = hero`, `push`, field assignment, return): requires `S ≤ T`; copies `T`'s fields only. A move happens only when `S ≡ T`.
- View (parameter, `use`, loop variable): requires `S ≤ T`; nothing copied; the static type of the view is `T`, `Type(view)` is `S`.

#### 3.2.4 Writing through a narrower type (`⊑w`)

A function that writes the leaf paths `P` of a parameter typed `T` (its **write-set**, the leaves actually written in its body, computed in `infer_body` and part of its interface; for a function value, every leaf below each declared `mutates` path) accepts an argument of type `S` only if, for every `p ∈ P`, `S.p ≡ T.p` and `S.p` is not `readonly` outside its type: `EF004` "`penalize` writes `score: Integer` into `player.score`, an `Integer(min = 0, max = 10)`". The same check runs for a mutated collection of supersets (`EF005` "`addJohn` mutates `humans`, so it needs an `Array(Human)`: make it generic"). Inside generic code, writes through a value of a type parameter (or of `Type(parameter)`) are recorded in the write-set in terms of the constraint's fields and checked at each call, with the actual type (`D5`).

#### 3.2.5 Diagnostics

| Code | Condition |
| --- | --- |
| `TY010`, `TY011`, `TY012` | duplicate field; field and method with one name; duplicate method |
| `TY013` | a default value refers to another field or `self` |
| `TY014` | object literal (or `.=`, or spread source) has a field the expected type lacks |
| `TY015` | object literal misses a required field |
| `TY016` | private field read, written or given outside its type; building a type with private fields without defaults outside it |
| `TY017` | field given twice in a literal |
| `TY018` | a field written after a spread with another type than the replaced field |
| `TY019` | unknown field on a value |
| `TY020` | field access on an optional without `?.` |
| `TY021` | field access on a named union without narrowing |
| `TY022` | write to a `readonly` field outside the type |
| `TY023` | write through an immutable place |
| `TY024` | assignment to a method |
| `TY025` | `==` on a non-comparable type |
| `TY026` | existing value missing a field (even optional) |
| `EF004`, `EF005` | write-set exactness (§3.2.4) |
| `FN040`, `FN041` | `self` not first / annotated / outside a type |

#### 3.2.6 Runtime semantics

Objects are values stored inline (a frame, a parent, a collection slot). Assignment copies; a copy of a value whose source is dead afterwards is a move (never observable). Narrowing copies drop fields. Defaults are evaluated at each creation. `==` is deep.

#### 3.2.7 JavaScript lowering

- An object is a plain object literal. **Properties are always emitted in the canonical shape order** (sorted by name), at every construction site and in every copy function, so every spelling of a shape shares one hidden class (`D57`). Printing uses the generated formatter of the static spelling, in declaration order.
- `copy$S$T(v)` is generated per (source, destination) when they differ, `copy$T` otherwise: reads only `T`'s fields, recursively copies owned values (objects, collections; strings and primitives are shared, they are immutable in JS). Never `{ ...v }`, never `structuredClone`. Copy elision: when neither side can be written afterwards (both `let`, or the source dead), the copy is the reference itself.
- Methods and associated functions are module-level functions `T$m(self, …)`, `T$g(…)`, called statically. Function-pointer fields are properties called as `o.f(…)` (they never use `this`).
- Private fields are properties named `$p_x` (`$` can't start a Fa identifier, so no collision); `readonly` is compile-time only.
- `eq$T(a, b)` generated per static type; `.=` is a sequence of assignments.
- Defaults are inlined at each construction site, evaluated in order.

#### 3.2.8 Native lowering

Per `compilation-with-llvm.md#objects-and-tuples` and `#structural-typing-on-llvm`: inline struct, fields in canonical order (alignment descending, then name); methods are static functions; function-pointer fields are `{ fn, env }`; `copy<Shape>`/`drop<Shape>`/`eq<Shape>`/`hash<Shape>` per shape, none for trivially copyable shapes; narrowing copy `copy<S → T>`; views by specialization (direct calls) or offset tables (function values); object literals built in their destination.

#### 3.2.9 Tests

```fa
-- ✓ obj-1: structural identity, static method dispatch → "a human named Zeus" / "an animal named Zeus"
type Human = {
	name: String
	function describe = (self): String => "a human named {self.name}"
}
type Animal = {
	name: String
	function describe = (self): String => "an animal named {self.name}"
}
let animal = Animal { name = "Zeus" }
let human: Human = animal
console.log(human.describe())
console.log(animal.describe())

-- ✓ obj-2: copy narrows, use views → { name = "John" } / "John"
type Human = { name: String }
type SuperHuman = { ...Human, powers: String }
let superHuman = SuperHuman { name = "John", powers = "unlimited" }
let human: Human = superHuman
console.log(human)
use viewed: Human = superHuman
console.log(viewed.name)

-- ✗ obj-3: TY019
use viewed: Human = superHuman
console.log(viewed.powers)

-- ✗ obj-4: TY014
type Point = { x: Integer, y: Integer }
let point: Point = { x = 12, y = 13, z = 14 }

-- ✗ obj-5: TY015
let point: Point = { x = 12 }

-- ✓ obj-6: optional field omitted in a literal → none
type Contact = { name: String, email: String? }
let contact: Contact = { name = "Ana" }
console.log(contact.email)

-- ✗ obj-7: TY026
type Contact = { name: String, email: String? }
type Human = { name: String }
let human = Human { name = "Bao" }
let other: Contact = human

-- ✓ obj-8: defaults evaluated per creation; field typed by its default → "Hello" / 2
type Greeter = { greeting = "Hello", count: Integer = 2 }
let g = Greeter {}
console.log(g.greeting)
console.log(g.count)

-- ✗ obj-9: TY013
type Bad = { a: Integer = 1, b: Integer = a }

-- ✓ obj-10: readonly written by its own method → 100
type Account = {
	readonly balance: Integer = 0
	function deposit = (self, amount: Integer(min = 0)) mutates self.balance => {
		self.balance += amount
	}
}
mutable account = Account {}
account.deposit(100)
console.log(account.balance)

-- ✗ obj-11: TY022
account.balance = 1000

-- ✗ obj-12: TY016 (read) — and a second file line `Counter { #count = 3 }` is TY016 too
type Counter = { #count: Integer = 0, function current = (self): Integer => self.#count }
let counter = Counter {}
console.log(counter.#count)

-- ✓ obj-13: a type with private fields is nominal, still satisfies a structural parameter (D3) → 0
type Counter = { #count: Integer = 0, label: String, function current = (self): Integer => self.#count }
type Labelled = { label: String }
function show = (value: Labelled) => console.log(value.label)
let counter = Counter { label = "0" }
show(counter)

-- ✗ obj-14: TY002 (another type with the same fields is not `Counter`: private fields make it nominal)
type Other = { #count: Integer = 0, label: String }
let c: Counter = Other { label = "x" }

-- ✓ obj-15: spread value with a replaced field → { host = "localhost", port = 8080, secure = false }
type Settings = { host: String, port: Integer, secure: Boolean }
let defaults = Settings { host = "localhost", port = 80, secure = false }
let custom: Settings = { ...defaults, port = 8080 }
console.log(custom)

-- ✗ obj-16: TY018
let custom: Settings = { ...defaults, port = "8080" }

-- ✗ obj-17: TY014 (D9: spread of a superset into a narrower literal)
type Hero = { ...Settings, alias: String }
let hero = Hero { host = "h", port = 1, secure = true, alias = "a" }
let s: Settings = { ...hero }

-- ✓ obj-18: .= is a list of assignments, references stay valid → 8080 / true
mutable settings = Settings { host = "example.com", port = 80, secure = false }
use port = settings.port
settings .= { port = 8080, secure = true }
console.log(port)
console.log(settings.secure)

-- ✗ obj-19: TY024
type MyType = { onClick: () => Void, function describe = (self): String => "x" }
mutable o = MyType { onClick = () => console.log("hi") }
o.describe = () => "other"

-- ✓ obj-20: function pointer replaced → "Value: 13"
type Counter = {
	value: Integer
	format: (value: Integer) => String
	function display = (self) => console.log(self.format(self.value))
}
mutable counter = Counter { value = 13, format = (value) => "{value}" }
counter.format = (value) => "Value: {value}"
counter.display()

-- ✗ obj-21: TY025
let a = Counter { value = 1, format = (v) => "" }
let same = a == a

-- ✗ obj-22: TY010, TY011
type T = { a: Integer, a: String }
type U = { b: Integer, function b = (self) => 1 }

-- ✗ obj-23: TY023
let point = Point { x = 1, y = 2 }
point.x = 3

-- ✓ obj-24: == compares the fields of the declared type → true
type Human = { name: String }
function same = (a: Human, b: Human) => a == b
console.log(same(SuperHuman { name = "A", powers = "x" }, SuperHuman { name = "A", powers = "y" }))

-- ✗ obj-25: EF004 (writing through a narrower type)
type Scored = { score: Integer }
type Player = { name: String, score: Integer(min = 0, max = 10) }
function penalize = (scored: Scored) mutates scored => {
	scored.score = -5
}
mutable player = Player { name = "Ana", score = 3 }
penalize(player)

-- ✓ obj-26: only the written leaves count → "Bob"
function rename = (scored: { name: String }) mutates scored.name => {
	scored.name = "Bob"
}
mutable player = Player { name = "Ana", score = 3 }
rename(player)
console.log(player.name)

-- ✗ obj-27: TY020
type Box = { inner: Point? }
let b = Box { inner = none }
console.log(b.inner.x)

-- ✓ obj-28: associated function through Self → 12
type Counter2 = {
	value: Integer
	function create = (start: Integer): Counter2 => Counter2 { value = start }
	function zero = (): Counter2 => Self.create(0)
}
console.log(Counter2.create(12).value)
```

### 3.3 Number types as types (only what the type relation needs)

The numeric rules themselves (literal adaptation, range arithmetic, overflow, result types) are in `values/numbers.md`; the type relation needs:

- `Int(a..b, s) ≤ Int(c..d, s')` iff `c ≤ a` and `b ≤ d`, for **reading and copying**. For a **view inside an aggregate** (an element of a collection, a field reached through a parameter or `use`), additionally `JS_BIGINT(s) == JS_BIGINT(s')` (`D2`, `CV005` "an `Array(Int64)` is not an `Array(Integer)` on every target: convert it with `map((x) => Integer(x))`"). Native views with a different width are handled by specialization (element conversion on load, `D54`).
- `Float(F32) ≤ Float(F64)` (`D11`); no other implicit conversion between kinds: `Integer`, `Float`, `BigInt`, `Percentage` are unrelated (`CV001`).
- Integer literal types adapt to any integer type containing them, to a float type and to `BigInt`; float literals to `Float32`/`Float64`; never to `Percentage` without `%`.
- `Character ≤ String` (a value conversion natively, `D12`).
- `Byte` is `UInt8`; `Integer` has the range of `Int64` with storage `Default`; `Int64` written explicitly has storage `I64` (`D2`).

### 3.4 Anonymous unions and optionals

#### 3.4.1 Normalization (`normalize_union`, run on every `|`, join, difference and substitution)

1. Flatten nested unions (anonymous and `T?`); a named union is **not** flattened (it is one member).
2. Drop `Never`.
3. **Absorption, non-object members only** (`D1`): drop a member `m` when another member `n ≠ m` satisfies `m ≤ n` and `m` is a literal, an integer range, `None`, an enum member, a named-union member, a `Fields` type or a collection. Object, tuple and function members are never absorbed: two object members where one satisfies the other are a distinguishability error, not a silent merge (`unions.md`: `{ name: String } | { name: String, age: Integer }` is an error).
4. Integer ranges of the **same storage** that touch or overlap merge into their hull (`Integer(min = 0, max = 5) | Integer(min = 6, max = 9)` → `Integer(min = 0, max = 9)`), so a literal union `0 | 1 | 2` is one range type; ranges with gaps stay separate members.
5. Members of a single named union (or enumeration) merge into the masked form (`NamedUnion(n, mask)`, `Enum(n, mask)`); a full mask is the declared type.
6. Sort by `TypeId`, dedupe. Zero members → `Never`; one → that member.

#### 3.4.2 Distinguishability (`check_distinguishable`, `TY040`)

Run on every union written by the user (annotation, `type`, generic argument, constraint) and on every join without expected type. Each member gets a **runtime kind** (`D7`):

| Kind | Members |
| --- | --- |
| `number` | `Integer`, ranged and sized integers up to 32 bits and their unsigned forms, `Float`, `Float32`, `Percentage`, enumerations, `Fields` with ≤ 32 flags, their literals |
| `bigint` | `BigInt`, `Int64`, `UInt64`, `Int128`, `UInt128` written explicitly, `Fields` with > 32 flags |
| `string` | `String`, `Character`, string literals |
| `boolean`, `none`, `function`, `range`, `task` | the obvious |
| `array` | `Array`, `GrowingArray`, `OrderedColumnar`, tuples (all are JavaScript arrays) |
| one kind per other collection constructor | `Set`, `Map`, `Pool`… (each a distinct JavaScript class or class-tagged object) |
| `object` | object types, named unions (their members carry a global tag), `CStruct` |

Two members are distinguishable iff: different kinds; or same kind and (literal values differ, or integer ranges don't overlap, or they are members/subsets of named unions — different named unions included, since their tags are global, `D8`); or both `object` and (each has a field the other lacks, or both have a field whose types are disjoint literal types). Two members of kind `number` that are not literals or disjoint ranges (`Integer | Float`, `Color | Integer`, `Color | Light`): `TY040` with the fix "use a named union". Two non-literal members of `array` kind (`Array(Integer) | Array(String)`, `Tuple(Integer) | Array(Integer)`): `TY040`.

**Storing a value into a union** (copy or view): the value's static type must satisfy **exactly one** member (`TY041` "a value that satisfies several members can't be stored in the union: copy it into the member type first"). A literal must match one member exactly (TY33).

#### 3.4.3 `is` and narrowing on types

`value is T` where `value`'s static type is an anonymous union, an optional, a named union or a masked subset: a runtime test, narrowing per `control-flow/conditionals#narrowing`. On a non-union static type the result is static: always true `WTY002`, never true `TY042`. `T` must be a type that can be tested at runtime: a member, a union of members, a named union, a literal, a range, `none`; a type that is not a member and not a union of members (e.g. `value is { name: String }` against `{ name: String, age: Integer } | { title: String }`) is `TY043` "test a member of the union".

#### 3.4.4 Optionals

`T?` is `Optional(T)` is `T | None`; `T??` is `T?`; `Optional(Void)` is `TY071`. A value of `T` satisfies `T?`. `?.` short-circuits the chain to its right; `??` takes the non-`None` part. Both apply only to optionals (`TY044` "`??` only applies to optionals; this is an error, use `catch`" when the left side is a failing call).

#### 3.4.5 Diagnostics

| Code | Condition |
| --- | --- |
| `TY040` | union members can't be told apart at runtime (suggest a named union) |
| `TY041` | a value satisfies several members of the target union |
| `TY042` / `WTY002` | `is` on a non-union is never / always true |
| `TY043` | `is` with a type that is not a member (or union of members) of the subject's union |
| `TY044` | `?.` / `??` on a non-optional |

#### 3.4.6 Runtime, JavaScript, native

- Runtime: a union value holds exactly one member; `is` and `when` decide it without reflection.
- JavaScript: untagged. `is` compiles to: `typeof v === "number"` / `"bigint"` / `"string"` / `"boolean"` / `"function"`, `v === null` (none), `Array.isArray(v)`, `v instanceof FaSet` (and the other collection classes, `D33`), `v instanceof FaRange`, then a literal compare (`===`), a range compare, a field presence test (`"age" in v`), a discriminant compare (`v.kind === "circle"`) or a named-union tag compare (`v.$t === 17`). The checker records, per test site, the minimal decision sequence (it proved it decides the member). Optional: `none` is `null`.
- Native: a union whose members are all literals or ranges of one scalar storage (`"north" | "south"`, `Integer(min = 0, max = 9) | Integer(min = 20, max = 29)`) is that scalar, with no tag (string literals: the index of the literal); any other union is `{ tag, payload }`, tag = index in the canonical member order (sorted by stable hash); niches for optionals (`compilation-with-llvm.md#unions-and-optionals`). Widening a union value to a bigger union re-tags by a constant table; a union passed by reference to a callee expecting a wider union specializes the callee; a mutated union parameter must be exact (`EF004`).

#### 3.4.7 Tests

```fa
-- ✓ un-1: narrowing by type → "string of length 5" / "number 3"
function describe = (value: String | Integer): String => {
	if value is String {
		return "string of length {value.length}"
	}
	"number {value}"
}
console.log(describe("hello"))
console.log(describe(3))

-- ✓ un-2: normalization → prints "String" / "Integer" / "Integer?"
console.log(@typeToString("north" | String))
console.log(@typeToString(Int8 | Integer))
console.log(@typeToString(Integer? | None))

-- ✗ un-3: TY040 (both numbers)
type Number = Integer | Float

-- ✗ un-4: TY040 (first member has no field of its own; D1: no absorption of objects)
type Person = { name: String } | { name: String, age: Integer }

-- ✓ un-5: discriminant → "circle"
type Shape = { kind: "circle", radius: Float } | { kind: "square", side: Float }
let shape: Shape = { kind = "circle", radius = 1.0 }
console.log(shape.kind)

-- ✗ un-6: TY041 (the value satisfies both members)
type A = { a: Integer }
type B = { b: Integer }
type AB = { a: Integer, b: Integer }
let both = AB { a = 1, b = 2 }
let u: A | B = both

-- ✓ un-7: literal union → "north"
type Direction = "north" | "south" | "east" | "west"
let d: Direction = "north"
console.log(d)

-- ✗ un-8: TY040 (two enumerations are both numbers)
type Color = Enum { red, green }
type Light = Enum { red, green }
type Either = Color | Light

-- ✓ un-9: two named unions are distinguishable (D8) → "api"
type ApiError = Errors { Timeout }
type StorageError = Errors { Full }
function describe = (error: ApiError | StorageError): String => if error is ApiError { "api" } else { "storage" }
console.log(describe(ApiError.Timeout))

-- ✗ un-10: TY042
let n = 3
if n is String { console.log("never") }

-- ✓ un-11: WTY002 only (warning) → "always"
if n is Integer { console.log("always") }

-- ✓ un-12: Never disappears from a union → "Integer"
console.log(@typeToString(Integer | Never))

-- ✓ un-13: optional of optional → "Integer?"
type MaybeInteger = Optional(Integer?)
console.log(@typeToString(MaybeInteger))

-- ✗ un-14: TY044
let x = 3
let y = x ?? 0

-- ✓ un-15: common field read on an anonymous union (D6) → 1.0
type Circle = { kind: "circle", size: Float }
type Square = { kind: "square", size: Float, side: Float }
let s: Circle | Square = { kind = "square", size = 1.0, side = 1.0 }
console.log(s.size)

-- ✗ un-16: TY040 (array kind twice)
type T = Array(Integer) | Array(String)

-- ✓ un-17: ranges that don't overlap → "low"
type Level = Integer(min = 0, max = 9) | Integer(min = 20, max = 29)
function level = (l: Level) => if l is 0..10 { "low" } else { "high" }
console.log(level(3))

-- ✓ un-18: when exhaustive over an optional → "none"
function show = (maybe: Integer?) => when maybe is {
	Integer => "some"
	none => "none"
}
console.log(show(none))
```

### 3.5 Named unions and `Errors`

#### 3.5.1 Syntax as parsed

`Union { Circle = { radius: Float }, Labelled = String, Point }` → `MemberIndex { target: Identifier("Union"), members: [Assignment { name, expression: type }, …, Identifier("Point")] }`. Same with `Errors`. Construction: `Shape.Circle { radius = 1.0 }` → `MemberIndex { target: Access [Shape, Circle], … }`; `Shape.Labelled("north")` → `FunctionCall { function: Access, parameters }`; `Shape.Point` → `Access`.

#### 3.5.2 Static semantics

- Member names unique (`TY050`). Inside the declaration, names refer to the enclosing scope (a member called `String` doesn't shadow the type `String`).
- Member forms: **literal** definition (`Hello = "hello"`): the literal is accepted directly where the union (or the member) is expected; two members can't accept the same literal (`TY051`). **Object** definition: built with `U.M { … }` (object literal rules of §3.2 against the payload). **Other type** (including a type name that denotes an object type: `Car = CarData`, `D4`): built with `U.M(value)`. **Bare**: `U.M`, no data.
- Wrong construction form: object member with parentheses `TY052`; non-object member with braces `TY053`; bare member called or given braces `TY054`; a plain object literal where the union is expected without the member name `TY055` "name the member: `Shape.Circle { … }`"; an unqualified member name in an expression or a pattern `TY057` (members are always qualified).
- Identity: nominal per declaration (and per generic application: `type Result(Value, Failure) = Union { Ok = Value, Failed = Failure }` is allowed; `Result(Integer, String)` is one type). Members of a generic named union are written `Result.Ok`, the arguments being inferred like those of a typed literal (expected type, then payload); `Result(Integer, String).Ok(3)` is the explicit form for when nothing gives them (`D62`).
- Members are types: `Member(n, i) ≤ NamedUnion(n, mask)` when `i ∈ mask`. A member with an object payload satisfies its payload object type (field reads, passing to a structural parameter); a member with a non-object payload does **not** satisfy its payload type — bind it with `as` (`D4`). A plain value of the payload type is never a member (`TY056`).
- Narrowing yields masked subsets; `when` exhaustiveness is a mask test.
- `Errors`: like `Union`, and every member satisfies `Error`. Only these may appear after `throws`: an `Errors` type, a member of one, a union of those, `Error`, the built-in errors (`IndexError`, `KeyNotFound`, `DivisionByZero`, `ArithmeticError`, `ParseError`, `Cancelled`, which are prelude `Errors` types), a type parameter (`Failure`). Anything else: `EF020`. `throw e` requires `e`'s type to satisfy `Error` (`EF021`).
- Difference: `Shape - Shape.Point` is `NamedUnion(n, mask without Point)`, same identity (§3.10).

#### 3.5.3 Runtime, JavaScript, native

- JavaScript (`D8`, `D43`): each member of each named union gets a **global tag id** (a small integer assigned per program, stable within one build). An object member is the payload object with an extra property `$t` (emitted first in property order, before the canonical fields); a non-object member is `{ $t, v }`; a bare member is a frozen module-level singleton `{ $t }`; a **literal member is the raw literal** (two members can't share a literal, so `v === "hello"` decides it). Thrown errors: per `values/errors.md` (JavaScript exception carrying the value).
- Native: `{ tag, payload }`, tag = declaration index, payload sized for the largest member; a single-member union has no tag (so `Union { User = Integer }` is an `i64`).

#### 3.5.4 Tests

```fa
-- ✓ nu-1: construction forms and when with as → "circle 1.0" / "label north" / "point"
type Shape = Union {
	Circle = { radius: Float }
	Labelled = String
	Point
}
function describe = (shape: Shape): String => when shape is {
	Shape.Circle as circle => "circle {circle.radius}"
	Shape.Labelled as text => "label {text}"
	Shape.Point => "point"
}
console.log(describe(Shape.Circle { radius = 1.0 }))
console.log(describe(Shape.Labelled("north")))
console.log(describe(Shape.Point))

-- ✓ nu-2: literal members accept the literal → "hello"
type MyUnion = Union { Hello = "hello", World = "world", Car = { brand: String } }
let greeting: MyUnion = "hello"
console.log(when greeting is {
	MyUnion.Hello => "hello"
	else => "other"
})

-- ✗ nu-3: TY051
type Bad = Union { A = "x", B = "x" }

-- ✗ nu-4: TY052 / TY053 / TY054 (one per line)
let a = Shape.Circle(1.0)
let b = Shape.Labelled { value = "x" }
let c = Shape.Point()

-- ✗ nu-5: TY055
let s: Shape = { radius = 1.0 }

-- ✗ nu-6: TY002 (a UserId is not an Integer)
type UserId = Union { User = Integer }
let userId = UserId.User(12)
let raw: Integer = userId

-- ✗ nu-7: TY056
type Shape2 = Union { Circle = { radius: Float } }
function area = (c: Shape2.Circle) => c.radius
area({ radius = 1.0 })

-- ✓ nu-8: a member with an object payload satisfies its payload (D4) → 2.0
type Round = { radius: Float }
function radiusOf = (r: Round) => r.radius
console.log(radiusOf(Shape2.Circle { radius = 2.0 }))

-- ✗ nu-9: exhaustiveness (when error, owned by the control-flow brief): Shape.Point not handled
let x = when Shape.Point is {
	Shape.Circle => 1
	Shape.Labelled => 2
}

-- ✗ nu-10: TY057
let y = when Shape.Point is {
	Point => 1
	else => 2
}

-- ✗ nu-11: TY002 (nominal: same members, different unions)
type A = Union { X }
type B = Union { X }
let b: B = A.X

-- ✓ nu-12: Errors members satisfy Error → "ApiError.NetworkError { message = \"lost\" }"
type ApiError = Errors { NetworkError = { message: String }, Timeout }
let e: Error = ApiError.NetworkError { message = "lost" }
console.log(e.toString())

-- ✗ nu-13: EF020
function f = (): Integer throws String => 1

-- ✓ nu-14: generic named union, members written without arguments (D62) → "ok 3"
type Result(Value, Failure) = Union { Ok = Value, Failed = Failure }
let r: Result(Integer, String) = Result.Ok(3)
console.log(when r is {
	Result.Ok as v => "ok {v}"
	Result.Failed => "failed"
})

-- ✗ nu-14b: GN015 (`Failure` can't be inferred without context)
let r2 = Result.Ok(3)

-- ✓ nu-15: member names don't shadow types inside the declaration → "x"
type Weird = Union { String = String }
let w = Weird.String("x")
console.log(when w is { Weird.String as s => s })
```

### 3.6 Recursive types

#### Rules (T7)

- Recursion only through named `type` declarations (several may refer to each other).
- Every cycle of the type graph must pass through at least one **indirection edge**: the payload of a union member (named or anonymous, including `T?`) or the element of a collection. A cycle made only of inline object fields or tuple elements: `TY060` "`Node` contains itself".
- A generic declaration may refer to itself only with its own parameters, in order: `type List(Item) = { head: Item, tail: List(Array(Item))? }` is `GN020` (polymorphic recursion).
- Equality is coinductive (`Node` ≡ `Link = { value: String, child: Link? }`).

#### Canonicalization algorithm (makes coinductive equality an id compare)

1. When `type_decl(D)` finds a reference to a declaration currently being built, it records a placeholder; the SCC of mutually recursive declarations (Tarjan over declaration references) is built together.
2. Build a labeled graph: one node per structural type constructor occurrence; labels = kind + field names / member indices / collection kind + **non-recursive** child `TypeId`s; edges to recursive children.
3. Minimize with partition refinement (Hopcroft; SCCs are tiny, Moore is fine) so bisimilar nodes merge.
4. Canonically number the minimized graph by a deterministic DFS from each declaration's root (children in label order), hash the numbered graph, and intern: each node becomes a `Rec(RecId)` whose `RecInfo` stores its `TypeKind` with `Rec` children. Two declarations whose unfoldings are equal get the same root `TypeId`.
5. Each recursive edge that closes a cycle is marked **boxed** (native: the union member's payload or the optional's value is a compiler-made heap block owned by its parent; `none` is the null pointer niche).

#### Runtime, JavaScript, native

JavaScript: nothing special (objects are references internally, copies are deep by the generated copy function, which is **iterative** for recursive shapes to avoid stack overflow on long chains — same rule as native). Native: boxed edges, iterative `copy`/`drop` (`compilation-with-llvm.md#recursive-types`). `DeepKeys` stops at a recursive field; `@typeToSchema` emits `Schema.Reference`.

#### Tests

```fa
-- ✗ rec-1: TY060
type Node = { value: String, child: Node }

-- ✓ rec-2: through an optional; coinductive equality → "b"
type Node = { value: String, child: Node? }
type Link = { value: String, child: Link? }
let node = Node { value = "a", child = Node { value = "b", child = none } }
let link: Link = node
console.log(link.child?.value)

-- ✓ rec-3: through a named union → 3
type Tree = Union { Leaf = { value: Integer }, Branch = { left: Tree, right: Tree } }
function sum = (tree: Tree): Integer => when tree is {
	Tree.Leaf as leaf => leaf.value
	Tree.Branch as branch => sum(branch.left) + sum(branch.right)
}
console.log(sum(Tree.Branch { left = Tree.Leaf { value = 1 }, right = Tree.Leaf { value = 2 } }))

-- ✓ rec-4: through a collection → 2
type Folder = { name: String, folders: Array(Folder) }
let root = Folder { name = "/", folders = [{ name = "a", folders = [] }, { name = "b", folders = [] }] }
console.log(root.folders.length)

-- ✗ rec-5: GN020
type List(Item) = { head: Item, tail: List(Array(Item))? }

-- ✓ rec-6: mutual recursion between declarations → "ok"
type A = { b: B? }
type B = { a: A? }
let a = A { b = B { a = none } }
console.log("ok")

-- ✓ rec-7 (stress): a 1_000_000-long chain copies and drops without stack overflow → 1000000
-- build `mutable head: Node? = none` in a loop prepending, copy it, count the copy's length iteratively
```

### 3.7 Special types: `Never`, `None`, `Void`, `Error`

- `Never`: no value; `Never ≤ T` for every `T`; type of `throw`, `return`, `break`, `continue`, a `loop` without `break`, a call returning `Never`. A function declared `: Never` whose body can complete normally: `FN012`. `Integer(min = 1, max = 0)` is `Never`.
- `None`: one value, `none`.
- `Void`: a unit type, the return type of a function returning nothing. Allowed only as a return type, as `Task(Void)`, as a type argument that ends in a return position. `let x = log("done")` (or `mutable`, `use`, a field, a collection element, `Optional(Void)`, a union member): `TY070` "a `Void` result can't be stored". A generic instantiation that would store `Void` (`items.map((x) => console.log(x))` gives `Array(Void)`): `TY071` "the callback returns nothing: use `for`" (`D41`).
- `Error`: any error; only `toString()` is available (`TY019` for anything else; narrow first).
- There is no `Any` (`TY007` on `Any`, with "use a generic parameter or a union").

```fa
-- ✓ sp-1: Never disappears from a join → 1
function fail = (): Never => {
	loop { }
}
let value = if true { 1 } else { fail() }
console.log(value)

-- ✗ sp-2: TY070
function log = (message: String) => console.log(message)
let result = log("done")

-- ✗ sp-3: TY071
let results = [1, 2].map((x) => console.log(x))

-- ✗ sp-4: FN012
function stop = (): Never => console.log("returns")

-- ✗ sp-5: TY007 (no Any)
let x: Any = 3

-- ✓ sp-6: Void as a type argument → "done"
async function work = () => console.log("done")
let task: Task(Void) = start work()
await task
```

### 3.8 Enumerations

#### Syntax as parsed

`Enum { red, green = 2 }` → `MemberIndex { target: Identifier("Enum"), members: [Identifier, Assignment { name, expression: Integer }] }`. `.red` with an expected type is the shorthand member access (parser: verify a leading-dot access parses; it is used in `enumerations.md`).

#### Static semantics

- Members unique (`TY080`). Pinned values: all members or none (`TY081`), distinct (`TY082`), non-negative integer literals (`TY083`). Unpinned: `0, 1, 2…` in declaration order.
- Nominal per declaration. `Enum(n, ALL)` is the type; `EnumMember(n, i)` is the type of `Color.red` (a literal-like type). Narrowing yields masks.
- `.red` resolves against the expected type (an enumeration, an optional of one, a union containing exactly one enumeration with that member); otherwise `TY084` "write `Color.red`: the type of `.red` is not known here".
- Operations: `==`, `!=`, `is`, `when` (exhaustive over members), `Integer(c)` (exact, never fails), `c.toString()` (the member name), `Color.members: Array(Color)` (declaration order, a static constant), `Color.parse(text: String): Color throws ParseError` (exact, case-sensitive name match, `D13`). No ordering (`<` is `TY085`, `D13`), no arithmetic.
- Storage: the smallest unsigned integer holding the largest value.
- Runtime kind `number` (§3.4.2).

#### JavaScript / native

JavaScript (`D56`): a member is its integer value (a number); `Color.members` is a frozen array; `toString` indexes a generated name table (or a `switch` when pinned values are sparse); `parse` is a generated `switch` on the string. Native: the smallest unsigned integer; `members` and the name table in read-only data; `parse` a generated perfect-hash or length-then-compare switch.

#### Tests

```fa
-- ✓ en-1 → "red" / 0 / "red" "green" "blue"
type Color = Enum { red, green, blue }
let favorite: Color = .red
console.log(favorite.toString())
console.log(Integer(Color.red))
for color in Color.members {
	console.log(color.toString())
}

-- ✓ en-2: pinned values → 9
type Status = Enum { pending = 0, active = 1, archived = 9 }
console.log(Integer(Status.archived))

-- ✗ en-3: TY081
type Status = Enum { pending = 0, active }

-- ✗ en-4: TY082
type Status = Enum { a = 1, b = 1 }

-- ✗ en-5: TY083
type Status = Enum { a = -1, b = 2 }

-- ✓ en-6: parse with catch → "green" / "red"
let ok = Color.parse("green") catch error => Color.red
let bad = Color.parse("purple") catch error => Color.red
console.log(ok.toString())
console.log(bad.toString())

-- ✗ en-7: TY002 (enumerations are nominal)
type Light = Enum { red, green, blue }
let l: Light = Color.red

-- ✓ en-8: exhaustive when with shared branch → "cool"
let t = when Color.blue is {
	Color.red => "warm"
	Color.green | Color.blue => "cool"
}
console.log(t)

-- ✗ en-9: TY085
let less = Color.red < Color.blue

-- ✗ en-10: TY084
let c = .red
```

### 3.9 `Fields` and key sets

#### Syntax as parsed

`Fields { read, write, nested { a, b } }` → `MemberIndex { target: Identifier("Fields"), members: [Identifier, …, MemberIndex { target: Identifier("nested"), members }] }`. A field-set value `Permissions { read }` is a `MemberIndex` on the type; a key set `{ foo, nested { bar } }` is a `Members` node, read as a key set only where a `Fields` type is expected (expected type, argument of a key-taking type function, a lowercase parameter typed `Keys(…)`/`DeepKeys(…)`), as an object literal everywhere else.

#### Static semantics

- Flags unique per level (`TY090`). Structural identity: the **ordered** flag tree (`D14`): `Fields { a, b }` and `Fields { b, a }` are different types (their bits differ). At most 128 leaves (`TY091`).
- Leaves get bits in declaration order, a group's members replacing the group (`Fields { foo, bar, nested { baz1, baz2 } }`: `foo = 1, bar = 2, nested.baz1 = 4, nested.baz2 = 8`). Storage: the smallest unsigned integer with that many bits (12 flags → 16 bits).
- Values: `T { … }` or a key set where `T` is expected; `{}` is the empty set; naming a group names all its leaves; a group may list a subset (`nested { baz1 }`). Unknown flag: `TY092`.
- `T.flag` (and `T.nested.baz1`, `T.nested`) is a value of `T`. Operators: `a + b` (union), `a - b` (difference, a missing flag is not an error at value level), `==`, `x is in s` (all leaves of `x` in `s`, `D16`), `Integer(s)`, `s.toStringArray(): Array(String)` (dotted leaf paths in bit order). Mixing two different `Fields` types: `TY002`.
- Key sets are compile-time values (`ConstValue::KeySet`) when written literally or declared `static`; a key set of a `Keys(T)`/`DeepKeys(T)` type must name only keys of `T` (`TF010` "`age` is not a key of `User`").
- **Normalization** (`D15`): a group whose every leaf is present is the group; two key sets with the same leaves are the same value. So `Omit(T, { foo { bar, baz } })` equals `Omit(T, { foo })` when `foo` has exactly `bar` and `baz`.

#### `@for` over a key set

`@for key in keys { body }` where `keys` is a compile-time key set: unrolled at compile time, one copy of the body per **leaf** in declaration order; `key` is a compile-time path (`"nested.foo"` when interpolated); `value[key]` reads the field at that path with its own type. The body is checked once per key after unrolling; an error names the key and the application that caused it (`TF011`).

#### JavaScript / native

JavaScript: ≤ 32 flags: a number, `+` is `|`, `-` is `& ~`, `is in` is `(s & x) === x`, `>>> 0` after `~` (`D14`); > 32 flags: a `bigint` with the same operators. `toStringArray` loops over a generated name table. Native: the unsigned integer; same bit operations.

#### Tests

```fa
-- ✓ fi-1 → 3 / true / 1
type Permissions = Fields { read, write, admin }
let editor = Permissions { read, write }
console.log(Integer(editor))
console.log(Permissions.write is in editor)
console.log(Integer(editor - Permissions.write))

-- ✓ fi-2: groups → 15 / ["foo", "bar", "nested.baz1", "nested.baz2"] / true
type MyFields = Fields { foo, bar, nested { baz1, baz2 } }
let all = MyFields { foo, bar, nested { baz1, baz2 } }
console.log(Integer(all))
console.log(all.toStringArray())
console.log(MyFields { nested } == MyFields { nested { baz1, baz2 } })

-- ✗ fi-3: TY092
let p = Permissions { read, delete }

-- ✓ fi-4: structural → 1
type Rights = Fields { read, write, admin }
let r: Rights = Permissions { read }
console.log(Integer(r))

-- ✗ fi-5: TY002 (D14: order is part of the identity)
type Swapped = Fields { write, read, admin }
let s: Swapped = Permissions { read }

-- ✓ fi-6: @for over a key set compiles to three logs → "foo = 12" / "baz = 14" / "nested.foo = 15"
function logProperties(Value, keys: DeepKeys(Value)) = (value: Value) => {
	@for key in keys {
		console.log("{key} = {value[key]}")
	}
}
type Settings = { foo: Integer, bar: Integer, baz: Integer, nested: { foo: Integer, bar: Integer } }
let settings: Settings = { foo = 12, bar = 13, baz = 14, nested = { foo = 15, bar = 16 } }
logProperties(Settings, { foo, baz, nested { foo } })(settings)

-- ✗ fi-7: TF010
logProperties(Settings, { qux })(settings)

-- ✓ fi-8: empty set → 0
let nothing: Permissions = {}
console.log(Integer(nothing))
```

### 3.10 Type operations: spread, `|`, `-`, `?`

#### Spread (in an object type)

- Only object types (including `Pick`/`Omit` results and type parameters whose constraint is an object type) can be spread (`TY100` "only object types can be spread").
- Copies the public data fields with their defaults, `readonly` and `reactive`; not methods, not `#private` fields.
- Same field twice with the same type (spread or written): kept once. Different types: `TY101` "the type of `foo` is ambiguous", including against a hand-written field (no override).
- Field order (for printing and schemas): as written, spreads included.
- A spread of a type parameter (`...Entity`) is resolved at application; a conflict is reported at the application site with a note at the declaration (`TY101`).
- There is no intersection operator (`&` is a parse error).

#### Union `A | B`

§3.4. The sides must be distinguishable (`TY040`).

#### Difference `A - B`

- Left side: an anonymous union, a named union (or masked subset), an enumeration, an optional, a literal union, or an integer range type (`TY110` "`String` can't be split: no type means every string but one" for anything else, e.g. `String - "hello"`, an object type).
- Each member of the left side that satisfies the right side (or one of its members, when it is a union) is removed. A right-side member that removes nothing: `TY111` "`X` is not a member of `A`".
- Integer ranges: removing values or ranges keeps the remaining ranges (`Integer(min = 0, max = 10) - 0` is `Integer(min = 1, max = 10)`; `Integer(min = 0, max = 10) - 5` is `Integer(min = 0, max = 4) | Integer(min = 6, max = 10)`).
- A named union keeps its identity (masked form). Removing everything gives `Never`.
- `A - B` where `A` is a type parameter: evaluated at application (type functions) or rejected in a generic body signature (`GN012`).

#### Optional `T?`

`T | None`, idempotent.

#### Precedence

`?` > `-` > `|`. `TypeA | TypeB - TypeB` is `TypeA | (TypeB - TypeB)` = `TypeA`.

#### Tests

```fa
-- ✓ op-1 → "{ foo: String, bar: Integer }"
type TypeA = { foo: String }
type TypeB = { ...TypeA, bar: Integer }
console.log(@typeToString(TypeB))

-- ✗ op-2: TY101
type ParentA = { foo: String }
type ParentB = { foo: Integer }
type Child = { ...ParentA, ...ParentB }

-- ✓ op-3: Omit resolves the conflict → "{ foo: String }"
type Child = { ...ParentA, ...Omit(ParentB, { foo }) }
console.log(@typeToString(Child))

-- ✓ op-4: difference of literals → "\"world\" | \"fine?\""
type TypeA = "hello" | "world" | "you" | "fine?"
type TypeB = "hello" | "you"
console.log(@typeToString(TypeA - TypeB))

-- ✗ op-5: TY111
type T = ("a" | "b") - "c"

-- ✗ op-6: TY110
type T = String - "hello"

-- ✓ op-7: ranges → "Integer(min = 1, max = 10)"
console.log(@typeToString(Integer(min = 0, max = 10) - 0))

-- ✓ op-8: named union difference keeps identity
type Shape = Union { Circle = { radius: Float }, Point }
type Round = Shape - Shape.Point
let r: Round = Shape.Circle { radius = 1.0 }
let s: Shape = r -- valid: a Round is a Shape
console.log(when r is { Shape.Circle => "circle" }) -- exhaustive without Point → "circle"

-- ✓ op-9: precedence: `C` is `TypeA | (TypeB - TypeB)`, so `TypeA`
type C = TypeA | TypeB - TypeB
let a: TypeA = "world"
let c: C = a
console.log(c) -- world

-- ✗ op-10: TY100
type T = { ...Integer }

-- ✓ op-11: spread keeps defaults and readonly → 13
type Parent = { fieldA: Integer, fieldB = 13 }
type Child = { ...Parent, fieldC: String }
console.log(Child { fieldA = 1, fieldC = "c" }.fieldB)
```

### 3.11 Generics

#### 3.11.1 Parameters

- Declared right after the name: `type Name(P1, P2: C = D, n: T)`, `function name(P) = …`, methods `function map(Result) = (self, …)`.
- **Capitalized** name: type parameter; optional constraint `: C` (any type; a union means "one of these types", an object type means "has at least these fields", method requirements written as function fields with `self`: `{ describe: (self) => String }`); optional default `= D`.
- **Lowercase** name: compile-time value parameter; its annotation is the type of the value (required: `GN001`). Allowed value types: `Boolean`, integer types, `String`, `Character`, enumerations, `Fields` (key sets), and arrays, tuples, objects of them (`GN006` for `Float`, functions, collections other than arrays…).
- A constraint or default may reference earlier parameters (`select(Entity, keys: Keys(Entity))`); referencing a later one is `GN002`. Defaults must satisfy constraints (`GN003`). Duplicate names `GN004`. A parameter after one with a default must have a default (`GN005`).
- A lambda can't declare generic parameters (parse-level; `GN030` if a tree has them); a lambda inside a generic declaration uses the enclosing parameters.

#### 3.11.2 Generic application or call (T4, `D18`)

For a callee `C` with generic parameters, the first argument list `L` written after it is the **generic argument list** when:

1. `C` has at least one lowercase parameter (value parameters are never inferred); or
2. `L` is non-empty and **every** argument is a type expression: a name that resolves to a type or type parameter, a type application, an object *type* (`{ x: Integer }`, with `:`), a union, a function type, `Type(…)`. Literals (`"a"`, `3`), object literals (`{ x = 1 }`, `{}`), lambdas and value names are values (`D18`).

Otherwise `L` is the call, and the generic parameters are inferred. On a built-in scalar type (`Integer`, `Int8`, `Float`, `BigInt`, `Percentage`…): positional arguments are a **conversion** (§3.13), named arguments `min`/`max` a **refinement** (a type). `Integer(5)` is a conversion (of the literal), `Integer(min = 5)` a type.

#### 3.11.3 Constraints and binding

- `X: C` where `C` is a union: the argument binds **exactly one** member (T3): the unique member `m` with `Arg ≤ m`; zero → `GN010` "`String` doesn't satisfy the constraint `Integer | Float` of `Value`"; several → `GN011` (ambiguous). Inside generic signatures, the argument value is then viewed as the bound member: `Array(Integer(min = 0))` passed for `Array(Value)` with `Value = Integer` is a read-only view.
- `X: C` where `C` is an object type: binds the argument's **full** type (`Arg ≤ C`, else `GN010`). Method requirements: the actual type's declaration (the spelling of the argument) must have a method of that name whose signature fits (§4.8 rules, with `self` the receiver); missing → `GN013`.
- No constraint: any type except `Void` in a data position (`TY071` at use), and `Never` is allowed.

#### 3.11.4 Inference (call sites and typed literals)

Inputs: explicit arguments, expected type, argument types, defaults — in this priority (TY24). Algorithm per call:

1. Bind explicit generic arguments (and check constraints, `GN010`).
2. If there is an expected type, unify the declared return type (or, for a typed literal `Point { … }`, the type itself) with it; bindings found here are **fixed** (`let ratio: Float = sum([1, 2, 3])` fixes `Value = Float`).
3. Check each argument with the substituted parameter type as its expected type where that type is fully known (literals adapt: `[1, 2, 3]` becomes `Array(Float)`), otherwise infer the argument's type in *synthesis mode* (literals have their base type) and unify. Lambdas are checked last, once their parameter types are known (left to right among lambdas), so `map((x) => …)` infers `Result` from the body.
4. Unify collects, per unbound parameter, the set of candidate types. All candidates must be **equal** (after widening literals to their base type); two different candidates is `GN014` ("`NumberType` is `Integer` for `x` and `Float` for `y`"). Never a join: `Point { x = 12, y = 1.5 }` is an error, not `Point(Integer | Float)`.
5. Remaining parameters take their default; still unbound: `GN015` "`Item` cannot be inferred" (with the explicit form as a fix).
6. Check every constraint with the final bindings (`GN010`/`GN011`/`GN013`), then re-check the arguments against the substituted parameter types (literal adaptation, views, write-sets).

A generic function used as a value (`["Ada"].map(identity)`) is instantiated from the expected function type by the same unification on parameter and return types; without an expected function type, `GN015`. A literal given to a generic parameter gives it its full type (`findById([{ id = 1, name = "Ana" }], 1)` binds `Entity = { id: Integer, name: String }`).

#### 3.11.5 Checking generic bodies (T2)

A generic declaration is checked **once**, against its constraints:

- A value of type `X` with an object constraint has the constraint's fields and methods; with no constraint, only what every type has (copy, pass, store in a collection of `X`, `Type(…)`; not `==`, not `toString` interpolation — `GN016` "`Value` has no constraint allowing `*`" or "allowing `==`").
- With a **union** constraint (`D19`), an operation is checked once per member with `X := member`; it is valid iff it is valid for every member and the result type is either the same for every member or is that member itself (then the result is `X`). Effects are the union over members. Example: `value * 2` → `Integer` / `Float` → `X`; `value / 2` → `Float` / `Float` → `Float`; `value // divisor` → `throws DivisionByZero` (from the `Integer` member). A literal adapts to `X` when it adapts to every member (`mutable total: Value = 0`).
- `@for`, `@if` and type-function right sides are checked **after** substitution (per application); errors are reported at the application with a note at the declaration (`GN017`).
- Range proofs that depend on value parameters (`Integer(min = 0, max = rows - 1)`) use `SymInt` bounds: in the generic body they are treated as their storage type's full range (so an index proof can't use them); after monomorphization, native range analysis reruns and may remove checks, but **the `throws` set is fixed by the generic check** (target-independent).
- A value of type `X` is never `FLAT` in the reference checker (MR26).
- Writes through `X` values feed the write-set (`D5`).

#### 3.11.6 Method calls on type parameters

`item.describe()` with `item: X` resolves on the **actual** type at each instantiation (the spelling of the argument at the call site that bound `X`). The generic check only verifies the constraint declares it.

#### 3.11.7 `Type(parameter)` is an implicit generic

For a structural parameter `p: T`, `Type(p)` is `ImplicitParam(f, i)`, constrained by `T`. It appears when the body copies `p` without annotation, returns `p` from a single-expression body, writes `Type(p)`, or reflects on it. Such a function is checked like `function f(P: T) = (p: P, …)` and the parameter is marked **needs full type** in its summary (native: specialized on the full type; JavaScript: per-type emission or a hidden copy function, `compilation-with-llvm.md#javascript`). A `use` alias and a `for` variable keep the type of what they designate in the same way.

#### 3.11.8 Monomorphization versus erasure

| Target | Strategy |
| --- | --- |
| Native | Full monomorphization, demand-driven: instance key = (function, generic arguments, view or full type of each structural parameter, names reflected on). Lambdas passed to generic functions are part of the key (each lambda is its own type, so `map` is specialized and inlined). |
| JavaScript (`D20`) | Erasure by default. Exceptions, decided per type parameter from the summary: (a) a parameter with a **union constraint** whose members have different JavaScript behaviour (numbers vs bigints, overflow checks of `Integer` vs none for `Float`, integer `//` vs float `//`, literal `0` vs `0n`) gets **one emitted instance per bound member** (finite, at most the constraint's member count); (b) a parameter used for a **per-type operation** — a method call, an unannotated copy, `==`, hashing (`Set`/`Map` of `X`), formatting (`"{x}"`), `@typeToString(X)`, building a key set — receives a hidden **dictionary** argument (trailing, after requirements) with exactly the operations used (`{ copy, eq, hash, format, describe }`); (c) otherwise, nothing. A function value of a generic instance closes over its dictionary. |

#### 3.11.9 Diagnostics

| Code | Condition |
| --- | --- |
| `GN001` | lowercase generic parameter without a type annotation |
| `GN002` | constraint or default refers to a later parameter |
| `GN003` | default doesn't satisfy the constraint |
| `GN004` | duplicate generic parameter name |
| `GN005` | parameter without default after one with a default |
| `GN006` | compile-time parameter of a type without exact equality (`Float`, functions…) |
| `GN007` | wrong number of generic arguments |
| `GN008` | a type given to a lowercase parameter, or a value to a capitalized one |
| `GN010` | argument doesn't satisfy the constraint |
| `GN011` | argument satisfies several members of a union constraint |
| `GN013` | argument lacks a method required by the constraint |
| `GN014` | inference found two different types for one parameter |
| `GN015` | parameter can't be inferred |
| `GN016` | operation not allowed by the constraint |
| `GN017` | error in code checked after substitution (with the application site) |
| `GN020` | polymorphic recursion |
| `GN030` | generic parameters on a lambda |

#### 3.11.10 Tests

```fa
-- ✓ gn-1: inference from arguments and expected type → "Ada" / 6 / 6.0
function first(Item) = (items: Array(Item)): Item? => items.get(0)
function sum(Value: Integer | Float) = (values: Array(Value)): Value => {
	mutable total: Value = 0
	for value in values {
		total += value
	}
	total
}
console.log(first(["Ada", "Alan"]))
console.log(sum([1, 2, 3]))
let ratio: Float = sum([1, 2, 3])
console.log(ratio)

-- ✓ gn-2: explicit parameters → 6.0
console.log(sum(Float)([1, 2, 3]))

-- ✗ gn-3: GN010
let total = sum(String)(["a", "b"])

-- ✗ gn-4: GN015 then valid forms
function parse(Output: Integer | Float) = (input: String): Output throws ParseError => ...
let count = try parse("42")

-- ✗ gn-5: GN014
type Point(NumberType) = { x: NumberType, y: NumberType }
let mixed = Point { x = 12, y = 1.5 }

-- ✓ gn-6: Point(Float) adapts literals → "12.0"
let p = Point(Float) { x = 12, y = 13 }
console.log("{p.x}")

-- ✗ gn-7: GN016
function double(Value) = (value: Value) => value * 2

-- ✓ gn-8: union constraint checked per member (D19) → 4 / 3.0
function double(Value: Integer | Float) = (value: Value) => value * 2
console.log(double(2))
console.log(double(1.5))

-- ✓ gn-9: structural constraint binds the full type → "Ana"
type User = { id: Integer, name: String }
function findById(Entity: { id: Integer }) = (entities: Array(Entity), id: Integer): Entity? => {
	for entity in entities {
		if entity.id == id {
			return entity
		}
	}
	none
}
console.log(findById([User { id = 1, name = "Ana" }], 1)?.name)

-- ✓ gn-10: method constraint, resolved on the actual type → "I am a dog"
type Dog = { name: String, function describe = (self): String => "I am a dog" }
function describeAll(Item: { describe: (self) => String }) = (items: Array(Item)) => {
	for item in items {
		console.log(item.describe())
	}
}
describeAll([Dog { name = "Rex" }])

-- ✗ gn-11: GN013
type Cat = { name: String }
describeAll([Cat { name = "Tom" }])

-- ✓ gn-12: compile-time value parameters; equal values give one type
type Matrix(Element, rows: Integer, columns: Integer) = { values: Array(Element) }
type A = Matrix(Float, 4, 4)
type B = Matrix(Float, 2 + 2, 4)
let a: A = { values = [] }
let b: B = a -- valid: A ≡ B

-- ✗ gn-13: compile error on the out-of-range index (Matrix.get takes Integer(min = 0, max = rows - 1))
-- transform.get(4, 0) with transform: Matrix(Float, 4, 4) → TY002 at the call (4 doesn't satisfy Integer(min = 0, max = 3))

-- ✗ gn-14: GN006
type Bad(scale: Float) = { x: Float }

-- ✓ gn-15: generic application vs call → "abab"
function repeatText(times: Integer(min = 0)) = (text: String): String => {
	mutable result = ""
	for 0..times {
		result += text
	}
	result
}
console.log(repeatText(2)("ab"))

-- ✓ gn-16: empty() is a call, inferred from the expected type → 0
function empty(Item) = (): Array(Item) => []
let numbers: Array(Integer) = empty()
console.log(numbers.length)

-- ✗ gn-17: GN015
let nothing = empty()

-- ✓ gn-18: defaults → "{ index: Integer }"
type Collection(Item, Index = Integer) = { get: (index: Index) => Item? }
let names: Collection(String) = { get = (index) => none }
console.log(@typeToString(Parameters(Type(names.get))))

-- ✓ gn-19: generic function as a value, instantiated by the expected type → ["Ada", "Alan"]
function identity(Value) = (value: Value): Value => value
console.log(["Ada", "Alan"].map(identity))

-- ✗ gn-20: GN015 (no expected function type)
let f = identity

-- ✓ gn-21: generic methods → [2, 4]
type Stack(Item) = {
	#items: Array(Item) = []
	function push = (self, item: Item) mutates self => {
		self.#items.push(item)
	}
	function map(Result) = (self, transform: (item: Item) => Result) => Stack(Result) { #items = self.#items.map(transform) }
	function toArray = (self) => self.#items[..]
}
mutable stack = Stack(Integer) {}
stack.push(1)
stack.push(2)
console.log(stack.map((x) => x * 2).toArray())

-- ✗ gn-22: constraints using earlier parameters: TF010
type User2 = { name: String, email: String, password: String }
function select(Entity, keys: Keys(Entity)) = (entity: Entity): Pick(Entity, keys) => pick(entity, keys)
let u = User2 { name = "Ada", email = "a@b", password = "s" }
let bad = select(User2, { age })(u)

-- ✓ gn-23: … and the valid call → { name = "Ada", email = "a@b" }
console.log(select(User2, { name, email })(u))

-- ✗ gn-24: D5 write-set through a type parameter: EF004 at the call
type Tight = { name: "a" | "b" }
function renamed = (human: { name: String }, name: String): Type(human) => {
	mutable copy = human
	copy.name = name
	copy
}
let t = renamed(Tight { name = "a" }, "zzz")

-- ✓ gn-25: Type(parameter) keeps the full type → "unlimited"
type Human = { name: String }
type SuperHuman = { ...Human, powers: String }
let clark = renamed(SuperHuman { name = "John", powers = "unlimited" }, "Clark")
console.log(clark.powers)
```

### 3.12 Type functions

#### 3.12.1 User type functions

- Declared like generic types; the right side is any type-producing expression: object type, union, spread, application, `when`/`if`/`do`.
- **Checked when applied**, with the actual parameters; errors reported at the application, with a note at the declaration (`TF002`).
- **Memoized** on `(DeclId, args)`; more than **256 nested applications** (counted on the active application stack, not in total) is `TF001` "type function applications nest too deeply (256)".
- `@` calls inside a type function run the pure function at compile time (bounded at 100 million interpreter steps, `TF003`); a thrown error becomes a compile error at the application (`TF004`, message = the error's `toString()`).
- A type function can't produce a value and a `function` can't produce a type (`TF005`).

#### 3.12.2 Evaluation

`eval_type(expr, env) -> TypeId` in the type-mode interpreter, where `env` maps parameters to `TypeId`/`ConstId`. `if c { A } else { B }` evaluates `c` as a compile-time boolean (only value parameters, literals, `static` values and `@` calls may appear; else `TF006`). `do { … }` runs statements in the compile-time interpreter; its last expression must be a type.

#### 3.12.3 `when` on types (TY12)

`when V is { P1 => A, P2 => B, else => C }`: the first branch with `V ≤ Pi` (the **whole** type, not member by member) is taken. In the branch, `V` is known to satisfy `Pi` (its fields are usable). Without `else`, no matching branch is `TF007`. Example: `Optionalize(Integer?)` takes `else` and gives `Integer?`.

#### 3.12.4 Built-in type functions

| Function | Accepts | Result | Errors |
| --- | --- | --- | --- |
| `Type(value)` | any expression (not evaluated) | its inferred type without facts; a parameter / `use` / loop variable: the type of what it designates; spelling = declared or created spelling, else base type of the literal | `TF020` on a type (`Type(Integer)`) |
| `Keys(T)` | object type (or member with object payload) | `Fields` of its public data fields, declaration order | `TF021` not an object type (maps: use `KeyType`) |
| `DeepKeys(T)` | object type | recursive `Fields`; a recursive field is a leaf | `TF021` |
| `KeyType(M)` | `Map`, `GrowingMap` | key type | `TF022` |
| `Values(C)` | any collection | element type (map: value type) | `TF023` |
| `Partial(T)` | object type | each field `F` → `F?`, flags and defaults kept, **shallow** (`D21`) | `TF021` |
| `Required(T)` | object type | each field `F` → `F - None` (unchanged if no `None`); a field of type exactly `None` → `TF024` | `TF021`, `TF024` |
| `Record(keys, Item)` | key set (compile-time), any type | object with one field per key, nested groups → nested objects | `TF025` keys not a key set |
| `Pick(T, keys: DeepKeys(T))` | object type, key set of `T` | only those leaves, nested objects keep only picked children | `TF010` unknown key |
| `Omit(T, keys: DeepKeys(T))` | same | removes those leaves; a group (after normalization, `D15`) removes the field | `TF010` |
| `Parameters(F)` | function type | object `{ name: Type }` per parameter, in order; a variadic `...v: T` gives `v: Array(T)` | `TF026` unnamed or destructured parameter (`D22`), `TF027` not a function type |
| `Return(F)` | function type | return type (`Void` when none) | `TF027` |
| `Uppercase`, `Lowercase`, `Capitalize` | string literal, union of them, `String` | per literal; `String` → `String` (Unicode case mapping, locale-independent) | `TF028` other types |
| `TypeFromSchema(schema)` | compile-time `Schema` value | the described type (§3.14) | `TF040`–`TF043` |

Key-set arguments are compile-time values: written literally, `static`, or a lowercase parameter.

Value-level `pick(value, keys)` and `omit(value, keys)`: `keys` compile-time; result type `Pick(Type(value), keys)` / `Omit(…)`; builds a copy with those fields. If `value` is `static`, computed at compile time.

#### 3.12.5 Tests

```fa
-- ✓ tf-1: WithTimestamps → "{ id: Integer, title: String, createdAt: Integer, updatedAt: Integer }"
type WithTimestamps(Entity: { id: Integer }) = { ...Entity, createdAt: Integer, updatedAt: Integer }
type Article = WithTimestamps({ id: Integer, title: String })
console.log(@typeToString(Article))

-- ✗ tf-2: GN010 at the application (constraint)
type Bad = WithTimestamps({ title: String })

-- ✓ tf-3: when on types is not distributive → "Integer?" / "None" / "Integer?"
type Optionalize(Value) = when Value is {
	None => None
	else => Value?
}
console.log(@typeToString(Optionalize(Integer)))
console.log(@typeToString(Optionalize(None)))
console.log(@typeToString(Optionalize(Integer?)))

-- ✓ tf-4: if on a value parameter → "UInt8"
type Storage(size: Integer(min = 0)) = if size <= 255 { UInt8 } else { UInt64 }
console.log(@typeToString(Storage(100)))

-- ✗ tf-5: TF001
type Loop(N) = Loop(Array(N))
type X = Loop(Integer)

-- ✓ tf-6: Keys, DeepKeys, KeyType, Values
type MyType = { foo: String, nested: { bar: Integer } }
console.log(@typeToString(Keys(MyType)))       -- "Fields { foo, nested }"
console.log(@typeToString(DeepKeys(MyType)))   -- "Fields { foo, nested { bar } }"
console.log(@typeToString(KeyType(Map(String, Integer))))  -- "String"
console.log(@typeToString(Values(Map(String, Integer))))   -- "Integer"

-- ✗ tf-7: TF021
type K = Keys(Map(String, Integer))

-- ✓ tf-8: Partial / Required → "{ foo: String?, bar: Integer? }" / "{ foo: String, bar: Integer }"
type P = { foo: String, bar: Integer }
console.log(@typeToString(Partial(P)))
console.log(@typeToString(Required(Partial(P))))
let patch: Partial(P) = { bar = 12 }
console.log(patch.foo) -- none

-- ✓ tf-9: Record with nested keys → "{ foo: String, bar: String, nested: { baz: String } }"
console.log(@typeToString(Record({ foo, bar, nested { baz } }, String)))

-- ✓ tf-10: Pick / Omit nested → "{ foo: { bar: Integer } }" twice (D15 for the second)
type M = { foo: { bar: Integer, baz: Integer }, qux: String }
console.log(@typeToString(Omit(Pick(M, { foo }), { foo { baz } })))
console.log(@typeToString(Pick(M, { foo { bar } })))

-- ✓ tf-11: Omit of a whole group removes the field (D15) → "{ qux: String }"
console.log(@typeToString(Omit(M, { foo { bar, baz } })))

-- ✗ tf-12: TF010
type Q = Pick(M, { missing })

-- ✓ tf-13: Parameters / Return → "{ foo: String, bar: Integer }" / "String"
function someFunction = (foo: String, bar: Integer): String => foo
console.log(@typeToString(Parameters(Type(someFunction))))
console.log(@typeToString(Return(Type(someFunction))))

-- ✗ tf-14: TF026
type Bad = Parameters((Integer, Integer) => Integer)

-- ✓ tf-15: string type functions → "\"NORTH\" | \"SOUTH\"" / "String"
console.log(@typeToString(Uppercase("north" | "south")))
console.log(@typeToString(Uppercase(String)))

-- ✗ tf-16: TF028
type U = Uppercase(Integer)

-- ✓ tf-17: pick / omit on values → { foo = "foo" } / { bar = 12 }
let myObject = { foo = "foo", bar = 12 }
console.log(pick(myObject, { foo }))
console.log(omit(myObject, { foo }))

-- ✓ tf-18: Type(value) has no facts and doesn't evaluate → "Integer" / "String | Integer"
let five = 5
console.log(@typeToString(Type(five)))
function f = (value: String | Integer) => if value is String { @typeToString(Type(value)) } else { "" }
console.log(f("x"))

-- ✗ tf-19: TF004 (the compile-time function throws) "Only string literals are supported"
-- (types-at-runtime.md: `type Invalid = CapitalizeSecondLetter(Integer)`)

-- ✗ tf-20: TF007
type NoMatch(V) = when V is { String => Integer }
type X = NoMatch(Boolean)
```

### 3.13 Type conversion

#### Rules

| Situation | Mechanism | Checker rule |
| --- | --- | --- |
| Same shape, other name | nothing | `S ≡ T` |
| Superset to subset, copied | implicit | `Copy` mode: generated narrowing copy |
| Superset to subset, referenced | implicit view | `Read` mode, `Type(view)` keeps `S` |
| Object literal with extra field | — | `TY014` |
| Collection of supersets to a parameter | implicit if the parameter is not mutated; never for `Set` elements and `Map` keys | `CL010` / `EF005` |
| `T` to `T?` | implicit | union membership |
| `T?` to `T` | narrowing, `??` | `TY002` otherwise |
| Literal to a number type | adapts | literal adaptation |
| Integer range that fits | nothing | §3.3 |
| Integer to smaller range | the target type called with the value: `Int8(x)`, `Byte(x)` | proven: no error; else `throws ArithmeticError` |
| Between kinds | `Float(x)`, `BigInt(x)`, `Percentage(x)` / `(x)%` | explicit; mixing kinds `CV001` |
| `Percentage` to `Float` | `Float(p)` | |
| Value to string | interpolation, `.toString()` | no `String(x)` (`CV002`, `D46`) |
| String to number / enum | `Integer.parse`, `Float.parse`, `BigInt.parse`, `Color.parse` | `throws ParseError` |
| Enum / fields to integer | `Integer(x)` | exact |
| Float to integer | `Integer.round(x)`, `.floor`, `.ceil`, `.truncate` | `throws ArithmeticError` (NaN, infinity, out of range) |
| `BigInt` to an integer type | `Integer(b)`, `Int8(b)` | `throws ArithmeticError` unless proven (`D47`) |
| Between collections | `Target.from(source)` | always a copy |

Conversion calls on built-in scalar types are resolved by the "positional value on a scalar type" rule (§3.11.2). A conversion that is the identity (`Integer(n)` with `n: Integer`) is `WCV001` (useless). `as` never converts (`CV003` if `as` is used outside destructuring and `when`). Narrowing of a target integer type with a proven value: no `throws`.

#### Lowering

| Conversion | JavaScript | Native |
| --- | --- | --- |
| `Float(integer)` | identity (number); from an `I64`-storage integer `Number(b)` | `sitofp` |
| `Float32(x)` | `Math.fround(x)` | `fptrunc` |
| `Integer(enum)`, `Integer(fields)` | identity | `zext` |
| `Int8(x)` proven | identity | `trunc` |
| `Int8(x)` unproven | range test, `throw` `ArithmeticError` | compare + cold error return |
| `Integer(int64Value)` (`D2`) | `Number(b)` + ±2^53 range test (stop) | identity |
| `Int64(integer)` | `BigInt(n)` | identity |
| `BigInt(n)` | `BigInt(n)` | inline 63-bit tagged |
| `(x)%`, `Percentage(x)` | `x / 100` | `fdiv` |
| `Float(p)` | identity | identity |
| narrowing copy | `copy$S$T` | `copy<S → T>` |

#### Tests

```fa
-- ✓ cv-1 → 1.5 / 20.0 / 0.5 / 60%
let count: Integer = 3
let ratio: Float = 0.5
console.log(Float(count) * ratio)
console.log(40.0 * 50%)
console.log(Float(50%))
console.log(50% + 10%)

-- ✗ cv-2: CV001
let scaled = count * ratio

-- ✓ cv-3: integers compare by range
let small: Int8 = 100
let big: Int64 = small
let bounded: Integer(min = 0, max = 100) = 100
let byte: Byte = bounded
console.log(byte)  -- 100

-- ✗ cv-4: TY002 ("an Integer may not fit in a Byte, convert it with `Byte(...)`")
let b2: Byte = readNumber()

-- ✓ cv-5: unproven narrowing throws → 0
let n = 300
let s = Int8(n + readZero()) catch error => 0  -- readZero(): Integer returns 0
console.log(s)

-- ✓ cv-6: proven by narrowing, no try
function toByte = (value: Integer): Byte? => {
	if value < 0 or value > 255 {
		return none
	}
	value
}
console.log(toByte(12)) -- 12

-- ✗ cv-7: CV002
let t = String(42)

-- ✓ cv-8: parse → 42 / 8080
let parsed = Integer.parse("42") catch error => 0
console.log(parsed)
let port = Integer.parse("x") catch error => 8080
console.log(port)

-- ✗ cv-9: TY002 (Percentage is not a Float)
let r: Float = 50%

-- ✓ cv-10: collections convert with from → 3
let set = Set.from([1, 2, 2, 3])
console.log(Array.from(set).length)

-- ✗ cv-11: CV005 (D2: views between number and bigint integer storage)
function total = (values: Array(Integer)) => values.length
let longs: Array(Int64) = [1, 2]
total(longs)
```

### 3.14 Types at runtime

#### `@typeToString(T): String`

Compile-time. Prints the **spelling** at the use site: a name as written with its arguments (`UserId`, `Point(Integer)`); anonymous types canonically (`D49`): objects `{ x: Integer, y: Integer }` in declaration order of the spelling (spread order for spread-built types), unions in the order written (normalized members removed), literals in Fa syntax (`"north"`, `42`), ranged integers `Integer(min = 0, max = 10)` (only the bounds that differ from the storage), optionals `T?`, function types `(n: Integer) => Integer throws E requires r: R mutates p` (names kept, `async` prefix), collections `Array(Integer)`, `Fields { a, nested { b } }`, `Enum { … }` only for anonymous ones. `Type(value)` prints the declared or creating spelling, the base type for a literal. A composition prints its function type (`"(n: Integer) => Integer"`).

#### `@typeToSchema(T): Schema`

Builds a `Schema` value (prelude named union, exactly the table of `types-at-runtime.md`). Rules: a named type is wrapped in `Schema.Named { name, definition }`; a recursive use inside its own definition is `Schema.Reference { name }`; `Integer` is `Schema.Integer { bits = 64 }` (signed default true, `min`/`max` only when narrower than the storage); `Byte` is `Schema.Integer { bits = 8, signed = false }`; properties in declaration order with `isReadonly` and `default` (the literal type of a literal default, else `none`); anonymous unions in written order; named unions `Schema.NamedUnion { errors, members }`; functions `Schema.Function { parameters, returns, throws, async }` (requires and mutates are not described: `D49`).

#### `TypeFromSchema(schema)` (`D48`)

Inverse, at compile time. `Schema.Named` gives an alias name used for printing; `Schema.Reference` must refer to an enclosing `Named` (`TF040`); `Schema.Object` properties must have unique names (`TF041`); `Schema.Union` members must be distinguishable (`TY040`); `Schema.NamedUnion` and `Schema.Enum` create a nominal type whose identity is `(the applying type-function declaration, the schema ConstId)` — equal schemas give the same type; invalid values (`bits = 7`, `min > max` gives `Never`, not an error) `TF042`; `Schema.Function` builds a function type without effects other than `throws`; `Schema.Collection` with an unknown `kind` `TF043`.

#### Runtime tests

`is` and `when` on unions only (§3.4.6). No reflection beyond these operations.

#### Tests

```fa
-- ✓ rt-1 → "String" / "Integer" / "UserId" / "Point(Integer)" / "{ x: Integer, y: Integer }"
type UserId = Integer
type Point(N) = { x: N, y: N }
console.log(@typeToString(String))
let numbers = [1, 2, 3]
console.log(@typeToString(Type(numbers[0])))
console.log(@typeToString(UserId))
console.log(@typeToString(Point(Integer)))
console.log(@typeToString({ x: Integer, y: Integer }))

-- ✓ rt-2: schema of a type with a default and an optional → assertion holds
type MyType = { foo: String = "hello", bar: Integer? }
assert @typeToSchema(MyType) == Schema.Named {
	name = "MyType"
	definition = Schema.Object {
		properties = [
			{ name = "foo", definition = Schema.String, isReadonly = false, default = Schema.StringLiteral { value = "hello" } }
			{ name = "bar", definition = Schema.Union { items = [Schema.Integer { bits = 64 }, Schema.None] }, isReadonly = false }
		]
	}
}

-- ✓ rt-3: recursive schema uses Reference
type Node = { value: String, child: Node? }
let s = @typeToSchema(Node)  -- Named "Node" whose child is Union [Reference "Node", None]

-- ✓ rt-4: TypeFromSchema round trip → "\"hEllo\" | \"wOrld\"" (types-at-runtime.md example)

-- ✓ rt-5: composition type → "(n: Integer) => Integer"
function stringLength = (text: String): Integer => text.length
function toText = (n: Integer): String => n.toString()
function getDigitCount = toText then stringLength
console.log(@typeToString(getDigitCount))
```

---

## 4. Functions

### 4.1 Declarations

#### 4.1.1 Syntax as parsed

```
[async] function name[(Generics)] [: FunctionType] = FunctionExpression
FunctionExpression = (params)[: Return] [throws E] [requires r: R, …] [mutates p, …] [invalidates p, …] => body
                   | name | a then b then … | f with (n = v, …) | factory(args) | oneToFour[1..]
```

- `Node::Function { name, is_async, parameters /* generic */, expression }`; the expression is an `ArrowFunction { parameters, return_type_expression, errors, requirements, mutations, invalidations, body: Block | Expression }`, or any other expression (alias `Identifier`, `Compose`, `PartialApplication`, `FunctionCall`, `Index` with a range).
- Parameters: `Identifier`-typed `Assignment`s (`a: T = d`), `Spread` (variadic), `DestructuredParameter { pattern, type_expression }`, `Members` (anonymous structure parameter), `self`.
- Parser gaps: `function f: Type = …` (annotated declaration), `async` function types, `[..]` and `x[i].f` paths after `mutates`/`invalidates`.

#### 4.1.2 Static semantics

- **One declaration form.** `let f = (x) => x + 1`: `FN001` "use `function f = (x) => x + 1`" (a lambda as the value of any declaration: `let`, `mutable`, `use`, `static`, `reactive`, `derived`). `let f = add` (a function *name*) is allowed and gives a plain function value (`D28`).
- **Single-expression body** (anything after `=>` that doesn't start with `{`, including `if`/`when`/`do` with blocks inside): return type and effects (`throws`, `requires`, `mutates`, `invalidates`) are **inferred**; any written ones are checked as for a block (allowed, a declared extra is `WEF001`).
- **Block body**: return type and every effect must be declared. Missing effect: `EF001` (mutates), `EF002` (invalidates), `EF003` (requires), `EF010` (throws: "`saveScore` does not declare `ApiError`"). No return type: returns `Void`; the last expression's value is dropped, `WFN001` when computing it has no effect (a pure expression, not a call). A declared effect the body doesn't have: `WEF001`; callers still see it.
- **Return**: the value of the last expression of a block, or `return e`. With a declared return type `R`: every `return e` and the final expression must be `≤ R` (`TY002`); a path reaching the end without a value: `FN011` "missing value at the end of `f`"; `return` without value in a function returning `R ≠ Void`: `FN011`; `return e` in a function without return type: `FN013`. `: Never` with a reachable end: `FN012`. `return` outside a function (module-level `do`): `FN014`.
- **Single-expression return type**: the expression's type; when it is a parameter (or a path into one, D5 aside), `Type(parameter)` (structural-typing: "returns its full type").
- **Recursion**: §1; `FN010`.
- **Hoisting**: at the top level and inside types, order doesn't matter. Inside a block, a function is visible after its declaration, can call itself, can use values declared before it; two local functions calling each other: `FN015` "declare them at the top level".
- **Names**: no overloading; a second function with the same name in a scope is `TY009`. A local function can shadow an outer name.
- **Annotated declaration** `function ascending: Comparator = (a, b) => a - b`: the lambda takes its parameter types, return type and effects from `Comparator`; the declared function has `Comparator`'s signature (parameter names from the lambda, effects exactly those of the type: the type is the contract, so declaring more than the body does is not warned). The body's effects must fit the type (`FN060`).
- **async**: `async function` only; never inferred; `await` allowed only in async contexts (`FN016`). The rest of async is in `advanced/concurrency.md`.

#### 4.1.3 Tests

```fa
-- ✓ fd-1: inferred return type and effects → 4
function addOne = (x: Integer) => x + 1
console.log(addOne(3))

-- ✓ fd-2: block with return type → "negative" / "zero" / "positive"
function classify = (value: Integer): String => {
	if value < 0 {
		return "negative"
	}
	if value == 0 {
		return "zero"
	}
	"positive"
}
console.log(classify(-1))
console.log(classify(0))
console.log(classify(1))

-- ✓ fd-3: WFN001 (warning) and returns nothing
function addOne = (x: Integer) => {
	x + 1
}

-- ✗ fd-4: EF001
type Player = { name: String, score: Integer }
function reset = (player: Player) => {
	player.score = 0
}

-- ✓ fd-5: single expression infers `mutates names` → ["guest"]
function addGuest = (names: Array(String)) => names.push("guest")
mutable names: Array(String) = []
addGuest(names)
console.log(names)

-- ✗ fd-6: EF011 (inferred mutation needs a mutable argument)
let fixed: Array(String) = []
addGuest(fixed)

-- ✗ fd-7: FN001
let addOne = (x: Integer) => x + 1

-- ✗ fd-8: FN010
function countdown = (n: Integer) => if n == 0 { 0 } else { countdown(n - 1) }

-- ✓ fd-9: recursion with a declared return type → "ababab"
function repeatText = (text: String, times: Integer(min = 0)): String =>
	if times == 0 { "" } else { text + repeatText(text, times - 1) }
console.log(repeatText("ab", 3))

-- ✗ fd-10: EF010
type ApiError = Errors { Timeout }
function saveScore = (score: Integer) => {
	throw ApiError.Timeout
}

-- ✓ fd-11: WEF001 (declared, never thrown) → 1
function one = (): Integer throws ApiError => {
	1
}
console.log(one() catch error => 0)

-- ✗ fd-12: FN015
function main = () => {
	function a = (n: Integer): Integer => if n == 0 { 0 } else { b(n - 1) }
	function b = (n: Integer): Integer => a(n)
}

-- ✓ fd-13: hoisting at the top level → 2
console.log(later())
function later = () => 2

-- ✗ fd-14: FN011
function f = (flag: Boolean): Integer => {
	if flag {
		return 1
	}
}

-- ✗ fd-15: FN013
function g = (x: Integer) => {
	return x
}

-- ✓ fd-16: annotated declaration → -1
type Comparator = (a: Integer, b: Integer) => Integer
function ascending: Comparator = (a, b) => a - b
console.log(ascending(1, 2))

-- ✓ fd-17: a body that is an if with blocks is a single expression → "big"
function size = (n: Integer) => if n > 10 {
	let label = "big"
	label
} else {
	"small"
}
console.log(size(11))
```

### 4.2 Parameters and calls

#### 4.2.1 Passing semantics

- A parameter is an **immutable reference** to the argument: never copied (native: aggregates by pointer, scalars by value since they can't be mutated; JavaScript: the value, objects and collections being references).
- `mutates p` (or a path below `p`): the function writes through the reference; the caller's argument must be a **mutable place** (`EF011` "`addBonus` mutates its argument, but `alice` is immutable"); a temporary (a call result, a literal) is `EF012` (`D24`).
- A parameter can't be replaced (`p = …`: `EF013` "a parameter can't be replaced: return the new value"); a primitive parameter can't be mutated at all (`mutates value` on a primitive: `EF014`). Primitive = numbers, `Boolean`, `String`, `Character`, enums, `Fields`, literal unions, `None`, ranges, function values.
- The argument keeps its type: `Type(p)` (§3.11.7).
- Aliasing: a place passed to a mutated/invalidated parameter must not overlap other arguments, used requirements, used module-level values or references captured by callback arguments (`advanced/reference-checking#arguments-that-alias`; `EF030`).

#### 4.2.2 Defaults, named arguments, variadic, destructuring

- **Defaults** are evaluated at each call that omits the argument, in the callee, after the previous parameters are bound; they can reference earlier parameters (`end: Integer = text.length`), not later ones (`FN020`). Their effects are effects of the function (`D25`). A default must satisfy the parameter type (`TY002`).
- **Named arguments**: positional first, then named (`FN021` "a positional argument can't follow a named argument"); a parameter given twice (`FN022` "`x` is already given"); unknown name (`FN023`); missing required argument (`FN024`); too many positional arguments (`FN025`). A variadic parameter is never named (`FN026`). Named arguments and defaults only exist for direct calls of a declaration (and of `with` results); through a function value: `FN027` "named arguments don't go through function values".
- **Variadic** `...values: T`: at most one, last (`FN028`); inside it is `Array(T)` (immutable). At the call, the remaining positional arguments fill it; `...collection` spreads any collection whose elements satisfy `T` (copied), only into the variadic part (`FN029` elsewhere).
- **Destructured parameter** `({ x, y }: Point)`: one local per field, each a reference into the argument (like `use { x, y } = p`); renaming with `as`; defaults with `=`. The whole value has no name. With an inline anonymous type (`({ a, b }: { a: String, b: String })`): `WFN002`, formatter rewrites to the anonymous structure parameter.
- **Anonymous structure parameter** `({ name: String, greeting: String = "Hello" })`: the parameter type is the object type `{ name: String, greeting: String = "Hello" }`; each field is a local reference. An object literal argument may omit defaulted fields; an existing value must have them (`TY026`).
- **Scope**: parameters and the top-level declarations of the body share one scope; redeclaring a parameter name in the body's top level is `TY009`.

#### 4.2.3 Call checking (direct call of a declaration)

1. Resolve the callee: a declaration (function, method through the receiver's static spelling, associated function, generic application), or a value of a function type.
2. Generic arguments: §3.11.2, §3.11.4.
3. Bind arguments to parameters (positional, named, variadic, defaults) — §4.2.2 errors.
4. Check each argument against its parameter type: `Read` mode for non-mutated parameters (views allowed), `Write(write-set)` mode for mutated ones (§3.2.4), exact element types for `Set`/`Map` keys.
5. Callback arguments: effects of non-stored ones join the call (§4.5); stored ones must fit the parameter's function type exactly (§4.8).
6. Effects of the call: the callee's effect set with argument places substituted for `Param(i)` roots (`D24`: a temporary argument of a mutated parameter is `EF012`); `throws` of the call = callee `throws` ∪ throws of non-stored callbacks; the call is a **failing call** if that set is non-empty: it must be under `try` (the enclosing function's throw set grows) or the left operand of a `catch` (`EF015` "`loadScore` can fail with `ApiError`. Use `try` to propagate the error, or `catch` to handle it"). `requires`: each requirement must be provided by an enclosing `provide` of the same shape or be a requirement of the enclosing function (`EF016`).
7. Reference checking applies the substituted effects (second walk).

#### 4.2.4 JavaScript lowering

- `function name(a, b = default, ...rest) { … }` (JavaScript defaults have the same per-call, left-to-right semantics; `none` is `null`, never `undefined`, so a passed `none` doesn't trigger the default).
- Named arguments are resolved at compile time into positional order; an omitted middle argument passes `undefined` to trigger its default.
- Destructured parameters: a plain parameter plus property reads at each use (no JavaScript destructuring, which would copy primitives and break `mutates` of fields).
- Hidden trailing arguments, in this order: requirements (canonical order), generic dictionaries (§3.11.8), the copy function of a `Type(parameter)` argument for function values.
- Spread arguments: `...array` when the source is a JavaScript array; other Fa collections are iterables (`[Symbol.iterator]`), so `...set` works.

#### 4.2.5 Native lowering

Aggregates by pointer with the attributes of `compilation-with-llvm.md#how-a-proven-fact-reaches-llvm` (`readonly` unless mutated, `noalias` thanks to the aliasing rule), scalars by value, results in registers or `sret`; `throws` as a tagged result; requirements as hidden pointer parameters; variadic as an `Array(T)` header built by the caller (on the stack when it doesn't escape); defaults evaluated in a callee prologue, with a "given" bitmask argument when the default depends on earlier parameters (otherwise the caller evaluates it).

#### 4.2.6 Tests

```fa
-- ✓ pa-1: named arguments → "Moving to (1, 2) at speed 3" ×3
function move = (x: Integer, y: Integer, speed: Integer = 1) => {
	console.log("Moving to ({x}, {y}) at speed {speed}")
}
move(1, 2, speed = 3)
move(1, speed = 3, y = 2)
move(x = 1, y = 2, speed = 3)

-- ✗ pa-2: FN021
move(x = 1, 2)

-- ✗ pa-3: FN022
move(1, 2, x = 3)

-- ✗ pa-4: FN024
move(1)

-- ✓ pa-5: default uses an earlier parameter → "ell"
function slice = (text: String, start: Integer = 0, end: Integer = text.length) => text[start..end] catch error => ""
console.log(slice("hello", 1, 4))

-- ✗ pa-6: FN020
function bad = (start: Integer = end, end: Integer = 0) => start

-- ✓ pa-7: variadic and spread → 6 / 0 / 10
function sum = (...values: Integer): Integer => {
	mutable total = 0
	for value in values {
		total += value
	}
	total
}
console.log(sum(1, 2, 3))
console.log(sum())
let numbers = [1, 2, 3]
console.log(sum(0, ...numbers, 4))

-- ✗ pa-8: FN028
function bad = (...a: Integer, b: Integer) => b

-- ✗ pa-9: FN026
sum(values = [1])

-- ✓ pa-10: a Set spreads into a variadic → 6
let set: Set(Integer) = [1, 2, 3]
console.log(sum(...set))

-- ✓ pa-11: destructured and anonymous structure parameters → "Values: Hello 42" / "Hello, John!" / "Hi, Jane!"
type SomeObject = { foo: String, bar: Integer, baz: Boolean }
function someFunction = (prefix: String, { foo, bar }: SomeObject) => {
	console.log(prefix, foo, bar)
}
someFunction("Values:", { foo = "Hello", bar = 42, baz = true })
function greet = ({ name: String, greeting: String = "Hello" }) => {
	console.log("{greeting}, {name}!")
}
greet({ name = "John" })
greet({ name = "Jane", greeting = "Hi" })

-- ✓ pa-12: WFN002 (warning)
function greet2 = ({ name, greeting }: { name: String, greeting: String }) => console.log(name)

-- ✗ pa-13: EF014
function increment = (value: Integer) mutates value => {
	value += 1
}

-- ✗ pa-14: EF013
function replace = (player: Player) mutates player => {
	player = Player { name = "x", score = 0 }
}

-- ✗ pa-15: EF012 (D24: a temporary given to a mutated parameter)
function addBonus = (player: Player) mutates player => {
	player.score += 10
}
addBonus(Player { name = "Bob", score = 0 })

-- ✓ pa-16: mutation through a mutable value → 10
mutable bob = Player { name = "Bob", score = 0 }
addBonus(bob)
console.log(bob.score)

-- ✗ pa-17: TY009 (a body declaration reusing a parameter name)
function f = (x: Integer) => {
	let x = 2
}

-- ✗ pa-18: EF030 (two mutated parameters, same argument)
function swapScores = (a: Player, b: Player) mutates a.score, b.score => {
	let t = a.score
	a.score = b.score
	b.score = t
}
swapScores(bob, bob)

-- ✓ pa-19: passing a superset by reference: no copy, writes visible → 1
type Scored = { score: Integer }
function bump = (s: Scored) mutates s.score => {
	s.score += 1
}
mutable p = Player { name = "Ana", score = 0 }
bump(p)
console.log(p.score)
```

### 4.3 Effects

#### 4.3.1 The model

| Effect | Declared as | Inferred from | Propagates to the caller as |
| --- | --- | --- | --- |
| return type | `: R` | final expression and `return`s | the call's type |
| `throws E` | `throws A \| B` | `try` on failing calls and failing assignment targets, `throw` (errors of non-stored callback parameters are the caller's, §4.5) | a failing call |
| `requires r: T` | `requires name: T, …` | calls to functions with requirements not satisfied by a `provide` in scope; reading a requirement name | must be provided or declared |
| `mutates P` | paths | writes, calls with substituted `mutates`, writes through loop variables / `use` aliases into parameters, module-level values, requirements | substituted paths |
| `invalidates P` | paths | calls with substituted `invalidates` (built-in `delete`, `pop`…) | substituted paths |
| `async` | `async` keyword | never | — |
| purity | — | §4.4 | pure functions can run at compile time, in `derived`, be fused |
| stored bits | — | §4.5 | the call's callback rules |
| freshness | — | returned references all from `add` | identity "fresh" at the call |

#### 4.3.2 Inference algorithm (`infer_body`)

Walk the typed body once (with facts). Maintain:

- `throws: TypeId` accumulator (union): add the errors of each failing call that is under `try`, of each `throw e` (the type of `e`), of failing assignment targets under `try` (`try scores[index] = 0`), and of the handler bodies of `catch` (a `throw error` in a handler adds the narrowed type of `error` there, e.g. the `else` remaining members). Errors of a `catch`-covered call are removed (not added).
- `requires: Vec<(Sym, TypeId)>`: from calls and direct uses, minus what `provide` statements satisfy at that point.
- `mutates`, `invalidates`: for each write or effectful call, the place written, mapped to a path whose root is a parameter, `self`, a module-level value or a requirement (local roots dropped; a local `use` alias stands for the place it designates; a loop variable over a parameter's collection stands for `param[..]`; a reference into a relation target starts again at the target's collection, `reference-checking#places-and-paths`). Index terms: a literal, a parameter, `root.length`/`root.size` ± constant when the index is one of those (identity), otherwise `[..]`.
- `writes: per parameter, the set of leaf paths written` (write-set, §3.2.4).
- purity bits, escape bits, freshness, needs-full-type, JavaScript dictionary needs.

Normalize at the end (§2.4). For a **single-expression** function, the summary *is* the signature. For a **block** function, each summary item must be covered by the declaration (a declared path covers its sub-paths; `[..]` covers every index; `invalidates` covers `mutates` of the same path; implied effects of relations are covered by their direct effect): uncovered item → `EF001`/`EF002`/`EF003`/`EF010` naming the path and the line that causes it. Declared but absent → `WEF001`.

#### 4.3.3 Effect paths in signatures

Grammar (normative, `reference-checking#places-and-paths`): `root { .field | .#field | [index] | [range] | [..] }`, `root` = a parameter, `self`, a module-level value, a requirement name; `index` = literal | parameter | `root.length` | `root.size`, optionally `± integer`. Checks: the root exists (`EF040`); each step exists in the root's type (`EF041` "`player` has no field `scor`"); `[index]`/`[range]` only on collections with positions or maps (`EF042`); `.length`/`.size` per the collection (`EF043`: `length` only on `Array`, `GrowingArray`, `OrderedColumnar`, `String`; `size` on the others); `invalidates` on a path that names no element (`invalidates self.name`) is `EF044`; `mutates` of a primitive parameter as a whole: `EF014`; `mutates people` on a module-level value replaces it (allowed), `mutates people.size` only adds/removes. Paths in a signature may use private fields (`self.#nodes`) even in a public function.

#### 4.3.4 Call-site checks (summary)

| Check | Error |
| --- | --- |
| a mutated or invalidated argument is not a mutable place (immutable binding, `let` parameter of the caller without a covering `mutates`, immutable collection) | `EF011` |
| a mutated argument is a temporary | `EF012` (`D24`) |
| write-set exactness through a narrower type | `EF004` |
| a mutated collection of supersets | `EF005` |
| an argument overlaps a mutated/invalidated place | `EF030` (reference checker) |
| a mutation of a collection being iterated (direct, through a call's effects, implied relation effects) | `EF031` (reference checker) |
| a failing call without `try`/`catch` | `EF015` |
| `try` and `catch` on the same expression | `EF017` |
| `try` on an expression that can't fail | `WEF002` |
| `try` in a function that doesn't declare the error (block body) | `EF010` |
| unsatisfied requirement | `EF016` |
| calling a mutating method on an immutable receiver | `EF011` |
| use of an invalidated (or maybe invalidated) reference | `EF032` (reference checker; message names the invalidating operation and proposes `use x = …` again or `reuse`) |
| an index proven out of bounds (`array[3]` on a 3-item array, `Integer(min = 4)` index) | `CL060` (range analysis) |

#### 4.3.5 Lowering of effects

Effects are erased; they only drive checks and attributes. Native: `memory(...)`, `readonly`, `noalias`, `nounwind` per `compilation-with-llvm.md#fas-effects-are-llvms-function-attributes`; `throws` is a tagged result. JavaScript: `throws` is a JavaScript exception, `requires` hidden arguments, mutations are plain writes.

#### 4.3.6 Tests

```fa
-- ✓ ef-1: precise mutation paths → 0 / "Ana"
type Player = { name: String, score: Integer }
function resetScore = (player: Player) mutates player.score => {
	player.score = 0
}
mutable ana = Player { name = "Ana", score = 5 }
resetScore(ana)
console.log(ana.score)
console.log(ana.name)

-- ✗ ef-2: EF001 (only player.score is declared)
function resetScore2 = (player: Player) mutates player.score => {
	player.score = 0
	player.name = ""
}

-- ✓ ef-3: [..] paths
function resetAll = (players: Array(Player)) mutates players[..].score => {
	for player in players {
		player.score = 0
	}
}
mutable team = [Player { name = "A", score = 3 }]
resetAll(team)
console.log(team[0].score) -- 0

-- ✓ ef-4: a failing assignment target with try
function resetAt = (scores: Array(Integer), index: Integer) throws IndexError mutates scores[index] => {
	try scores[index] = 0
}
mutable scores = [1, 2]
resetAt(scores, 5) catch error => console.log("out of bounds") -- "out of bounds"

-- ✗ ef-5: EF015
type ApiError = Errors { Timeout }
function loadScore = (): Integer throws ApiError => 1
function printScore = () => {
	let score = loadScore()
}

-- ✗ ef-6: EF017
let s = try loadScore() catch error => 0

-- ✓ ef-7: partial handling rethrows the rest (inferred throws in a single expression) → 0
type ApiError2 = Errors { Timeout, Network }
function load2 = (): Integer throws ApiError2 => throw ApiError2.Timeout
function safe = () => load2() catch error => when error is {
	ApiError2.Timeout => 0
	else => throw error
}
console.log(safe() catch error => -1)

-- ✓ ef-8: requirements → "Hello, Ana!"
type Logger = { info: (message: String) => Void }
function greet = (name: String) requires logger: Logger => {
	logger.info("Hello, {name}!")
}
do {
	provide Logger = { info = (message) => console.log(message) }
	greet("Ana")
}

-- ✗ ef-9: EF016
greet("Bao")

-- ✓ ef-10: module-level value mutation declared with its path
-- (file people.fa: `export mutable: Pool(Person) = []`)
-- function register = (name: String) mutates people.size => { people.add({ name = name }) }

-- ✗ ef-11: EF001 (people = [] needs `mutates people`)
-- function register = (name: String) mutates people.size => { people = [] }

-- ✗ ef-12: EF041
function f = (player: Player) mutates player.scor => { }

-- ✗ ef-13: EF044
function g = (player: Player) invalidates player.name => { }

-- ✓ ef-14: invalidates implies mutates size
type Inventory = {
	items: Pool(String)
	function remove = (self, item: String in self.items) invalidates item => {
		self.items.delete(item)
	}
}

-- ✗ ef-15: EF011 (calling a mutating method on an immutable value)
type Counter = { value: Integer, function increment = (self) mutates self.value => { self.value += 1 } }
let c = Counter { value = 0 }
c.increment()
```

### 4.4 Purity

A function is **pure** iff its summary has no `throws`, no `requires`, no `mutates`/`invalidates` (on anything but its locals), it is not `async`, it reads no module-level `mutable` or `reactive` value, and it calls (directly or not) only pure functions and no impure platform primitive (the prelude marks `console.*`, clock, randomness, I/O, `std.http`… as impure). Mutating a captured local in a non-escaping closure is local to the enclosing function: the closure is not pure, the enclosing function may be. Computed in `infer_body`, fixpoint on SCCs (start: pure; becomes impure monotonically). Used by: compile-time calls (`@f(...)` on an impure function: `FN050` "`f` is not pure: it calls `console.log`"), `derived`, fusion of method chains (§5.0.6), compile-time function factories (§4.10.4).

```fa
-- ✓ pu-1: compile-time call of a pure function → 6
function add = (a: Integer, b: Integer) => a + b
static six = @add(2, 4)
console.log(six)

-- ✗ pu-2: FN050
function logSquare = (x: Integer) => console.log(x * x)
static nothing = @logSquare(2)
```

### 4.5 Functions as parameters (stored or not)

- For each parameter of a function type, `infer_body` computes **stored**: the parameter (or a closure capturing it, or a `with`/`then` value built from it) is assigned to a field, a collection element, a module-level value; returned; captured by an escaping closure; used by a started task; or passed to a **stored** parameter of another function. Fixpoint on SCCs (start: not stored). The bit is part of the interface (cache entry with the effects) and appears in `FnSig.passthrough`.
- **Not stored**: the argument may have any effect. At the call, the argument's effects (`throws`, `requires`, `mutates`/`invalidates` of the places it captures by reference — they are the caller's places —, impurity, `async`?) become effects of the call. Inside the callee, calling the parameter needs no `try` and contributes nothing to the callee's own `throws`; the callee can't `catch` those errors: a `catch` whose left operand's only failing calls are calls of non-stored parameters is `EF018` "`f`'s errors belong to the caller"; when mixed with the callee's own failing calls, the `catch` covers only its own (`D29`).
- **Stored**: the parameter's declared function type is the contract; the argument must fit it exactly (§4.8), effects included (`FN060` "`onClick` is `() => Void`, which doesn't declare the errors of `save`").
- In a **function type**, a function-typed parameter is always considered stored (`D26`).

```fa
-- ✓ fp-1: a non-stored callback's errors become the call's → [1, 2] / "bad"
let texts = ["1", "2"]
let numbers = texts.map((text) => try Integer.parse(text)) catch error => []
console.log(numbers)
let other = ["x"].map((text) => try Integer.parse(text)) catch error => {
	console.log("bad")
	[]
}

-- ✗ fp-2: EF015 (map throws what its callback throws)
function parseAll = (texts: Array(String)) => {
	let n = texts.map((text) => try Integer.parse(text))
}

-- ✗ fp-3: FN060
type Button = { label: String, onClick: () => Void }
function save = () throws ParseError => { }
function setHandler = (button: Button, handler: () => Void) mutates button.onClick => {
	button.onClick = handler
}
mutable button = Button { label = "ok", onClick = () => console.log("x") }
setHandler(button, () => try save())

-- ✓ fp-4: a user function with a non-stored callback, no try inside → 3
function twice = (action: () => Integer): Integer => action() + action()
function risky = (): Integer throws ParseError => try Integer.parse("1")
console.log(twice(() => 1) + (twice(risky) catch error => 0) - 1)  -- 2 + 2 - 1 = 3

-- ✗ fp-5: EF018
function guarded = (action: () => Integer): Integer => action() catch error => 0
```

### 4.6 Lambdas

- Syntax: `(params) => body`, parameters always parenthesized, optional annotations, optional `: R throws … mutates …` like a declaration. `ArrowFunction` node.
- Allowed wherever a value is expected (argument, field value, element, return value, right side of an assignment `counter.format = (v) => …`), **except** as the value of a declaration (`FN001`).
- **Contextual typing** (C1): the expected function type gives the parameter types, the return type and the effects. What it leaves open (a generic `Result`) is inferred from the body. Without an expected type, every parameter must be annotated (`FN061` "the type of `x` is not known: annotate it"), and the return type and effects are inferred from the body whatever its form.
- A block body takes the expected return type; with an expected `Void` (or none, if the expected type returns `Void`), the last value is dropped; with an open `Result`, the last expression is the value (`names.map((name) => { let t = name.trim(); t.length })`).
- A lambda may declare **fewer** parameters than the expected type (trailing ones ignored); more: `FN062`.
- A lambda's effects must fit the expected type if the target is stored (§4.5); if the target parameter is not stored, they flow to the call.
- `async` lambdas: a lambda is async when its expected type is (`async` written on a lambda without an async expected type: `FN063`).
- Not generic (`GN030`). To return an object literal from a lambda: `=> Point { … }`; `=> { … }` is a block.

```fa
-- ✓ la-1 → [2, 4, 6] / [4, 3]
let doubled = [1, 2, 3].map((x) => x * 2)
console.log(doubled)
let lengths = ["Ana  ", " Bo"].map((name) => {
	let trimmed = name.trim()
	trimmed.length + 1
})
console.log(lengths)

-- ✓ la-2: fewer parameters than the callback type → ["a", "b"]
type Item = { name: String }
console.log([Item { name = "a" }, Item { name = "b" }].map((item) => item.name))

-- ✗ la-3: FN061
function apply = (f) => 1

-- ✓ la-4: returning an object literal needs the type → { x = 1, y = 0 }
type Point = { x: Integer, y: Integer }
console.log([1].map((x) => Point { x = x, y = 0 })[0])

-- ✗ la-5: FN062
let r = [1].map((a, b, c) => a)
```

### 4.7 Closures and captures

#### 4.7.1 Escape

A closure (a lambda, or a function declared in a block and used as a value or called after capture) **escapes** when it may be called after the current call of the enclosing function or from another task: it is stored (field, collection, module-level value), returned, passed to a **stored** parameter, captured by another escaping closure, or used by a started task (`start do`, `start f(closure)`). Otherwise it doesn't escape (called directly, passed to non-stored parameters).

#### 4.7.2 Capture

| | Non-escaping | Escaping |
| --- | --- | --- |
| What is captured | the variables themselves (by reference), like JavaScript | a **copy** of each used value, taken at creation, owned by the closure |
| Sees later writes of the enclosing function | yes | no |
| May modify a captured `mutable` local | yes, no declared effect; the write is a write of the enclosing function's local (counts for its iteration and invalidation rules at the call that runs the closure) | no: `FN070` "`count` is captured by copy, since the closure escapes" |
| Parameters, `use` aliases, loop variables | references (uses the place) | copies of the designated values |
| `mutable use` alias | `FN071` (never captured) | `FN071` |
| Module-level values, requirements | not captured: used directly / resolved where the closure is called | same |
| Move-only values (`Task`, `destroy`) | by reference | `FN072` (`D45`) |
| References into a collection mutated by the call it's passed to | `EF030` (reference checker) | n/a (copies) |

The compiler skips a copy when no later write to the captured value can be observed (the source is a `let`, or it is never written after the closure's creation, or it is dead).

#### 4.7.3 Lowering

- JavaScript: an arrow function. Non-escaping: closes over the variables directly. Escaping: each captured value that needs a copy is copied into a fresh `const` in a block (or IIFE) wrapping the arrow's creation: `{ const greeting$1 = greeting; callbacks.push(() => …greeting$1…) }`; objects and collections use the generated copy function; `let`-bound and never-written sources need nothing.
- Native: `{ fn, env }`. Non-escaping lambdas passed to generic functions are specialized and inlined (no environment). Other non-escaping closures: environment in the caller's frame (pointers to the captured variables). Escaping: environment struct of the copies; ≤ 8 bytes and trivially copyable → in the `env` word; otherwise a heap block with a `{ size, align, copy, drop }` descriptor (Q4).

#### 4.7.4 Tests

```fa
-- ✓ cl-1: non-escaping closure sees the current value → "Bye, John!"
function main = () => {
	mutable greeting = "Hello"
	function greet = (name: String) => console.log("{greeting}, {name}!")
	greeting = "Bye"
	greet("John")
}
main()

-- ✓ cl-2: non-escaping closure modifies a captured local → 6
function sumOf = (numbers: Array(Integer)): Integer => {
	mutable total = 0
	function addToTotal = (value: Integer) => {
		total += value
	}
	for number in numbers {
		addToTotal(number)
	}
	total
}
console.log(sumOf([1, 2, 3]))

-- ✓ cl-3: escaping closures copy → "Hello, Ada!" / "Hi, Ada!"
function makeGreeter = (greeting: String) => (name: String) => "{greeting}, {name}!"
let greetings = [makeGreeter("Hello"), makeGreeter("Hi")]
for g in greetings {
	console.log(g("Ada"))
}

-- ✓ cl-4: an escaping closure keeps the value at creation → 1
function main2 = () => {
	mutable n = 1
	mutable callbacks: Array(() => Void) = []
	callbacks.push(() => console.log(n))
	n = 2
	for callback in callbacks {
		callback()
	}
}
main2()

-- ✗ cl-5: FN070
function main3 = () => {
	mutable count = 0
	mutable callbacks: Array(() => Void) = []
	callbacks.push(() => {
		count += 1
	})
}

-- ✗ cl-6: FN071
mutable team = [Player { name = "A", score = 1 }]
mutable use best = team[0]
let names = [1].map((x) => best.name)
```

### 4.8 Function types and assignability

#### 4.8.1 Syntax as parsed

`(a: T, b: U) => R throws E requires r: X mutates a.f invalidates a[i..]` → `ArrowFunction` (type mode) wrapped in `Throws`/`Requires`/`Effect`; `(T, U) => R` (unnamed parameters); `async (…) => R` (parser gap). `Void` must be written as the return type of a function type that returns nothing (`TY072` "write `=> Void`").

#### 4.8.2 Static semantics

- Identity: `FnSig` without parameter names (§2.2). Names are needed when an effect refers to a parameter (`EF040` otherwise).
- **`f` fits `T`** (assigning a function or a function value to a place of type `T`, passing it to a stored parameter, returning it as `T`):
  1. Same parameter count, matched by position (`FN080`); a lambda may declare fewer. A variadic parameter matches only a variadic parameter with the same element type (`FN085`, `D26`).
  2. Each `T.param[i] ≤ f.param[i]` (contravariant, `FN081`); if `f` mutates parameter `i`, its write-set must be exact against `T.param[i]` (`EF004`).
  3. `f.ret ≤ T.ret`, or `T.ret` is `Void` (the value is dropped) (`FN082`).
  4. Every effect of `f` is declared by `T` (`FN060`): `f.throws ≤ T.throws`; each requirement of `f` has a requirement of the same shape in `T`; each `mutates`/`invalidates` path of `f`, renamed by position (`mutates p` in `f` ↦ `mutates player` in `T` when both are parameter 0), is covered by a path of `T`; effects on module-level values must appear in `T` with the same root. `T` may declare more (no warning).
  5. `f` async ⇒ `T` async (`FN084`); a non-async `f` fits an async `T`.
- A **call through a value** passes arguments by position only (`FN027` for named), never uses defaults, and has `T`'s effects with the arguments substituted.
- A generic function used as a value is instantiated from `T` (§3.11.4). A **method** can be used as a value through its type, `Human.describe`, with `self` as the first positional parameter of type `Human` (`D27`); a bound method `human.describe` without call is `FN086` "write a lambda: `() => human.describe()`".
- Function values are not comparable (`TY025`) and have runtime kind `function`.

#### 4.8.3 Lowering

JavaScript: a JavaScript function; hidden trailing arguments for requirements and, when the declared parameter type needs it, the copy function of the actual argument (`compilation-with-llvm.md#javascript`). Native: `{ fn, env }`; a function used as a value gets a **view entry point** taking an offset table per structural parameter (`compilation-with-llvm.md#function-values`); requirements as hidden pointer parameters in the ABI of the value.

#### 4.8.4 Tests

```fa
-- ✓ ft-1: a function type that declares the mutation → 10
type Player = { name: String, score: Integer }
type Reset = (player: Player) => Void mutates player
function addBonus = (player: Player) mutates player => {
	player.score += 10
}
let reset: Reset = addBonus
mutable p = Player { name = "A", score = 0 }
reset(p)
console.log(p.score)

-- ✗ ft-2: FN060
let callback: (player: Player) => Void = addBonus

-- ✓ ft-3: contravariant parameters, Void accepts any return → "Ana"
type Named = { name: String }
function nameOf = (n: Named): String => n.name
let f: (player: Player) => Void = nameOf
f(Player { name = "Ana", score = 0 })
let g: (player: Player) => String = nameOf
console.log(g(Player { name = "Ana", score = 0 }))

-- ✗ ft-4: FN081 (a function taking a Player can't take any Named)
function useIt = (h: (n: Named) => String) => h({ name = "x" })
useIt((player: Player) => player.name)

-- ✗ ft-5: FN027
type Mover = (x: Integer, y: Integer) => Void
function move = (x: Integer, y: Integer) => console.log(x + y)
let m: Mover = move
m(x = 1, y = 2)

-- ✗ ft-6: FN080 on the last line
function takesTwo = (f: (a: Integer, b: Integer) => Integer) => f(1, 2)
takesTwo((a) => a)  -- valid: a lambda may declare fewer parameters
function one = (a: Integer) => a
takesTwo(one)  -- FN080

-- ✗ ft-7: FN084
async function fetchIt = (): Integer => 1
let sync: () => Integer = fetchIt

-- ✓ ft-8: a non-async function fits an async type
type Handler = async (request: String) => String
function echo = (request: String) => request
let handler: Handler = echo

-- ✓ ft-9: method as a value (D27) → ["a human named A"]
type Human = { name: String, function describe = (self): String => "a human named {self.name}" }
console.log([Human { name = "A" }].map(Human.describe))

-- ✗ ft-10: FN086
let d = Human { name = "A" }.describe
```

### 4.9 Methods, associated functions, function pointers

- `function` inside a type: a **method** when its first parameter is `self` (no annotation, `FN041`), an **associated function** otherwise. `name: FunctionType` is a function-pointer field (stored per instance, replaceable on mutable instances).
- Methods are not stored, can't be replaced (`TY024`), can't be declared in anonymous objects (`FN042` "methods can only be declared in `type` declarations"). A field and a method can't share a name (`TY011`).
- `self` is an immutable reference to the receiver; `mutates self` / `mutates self.f` make the method callable only on a mutable place (`EF011`). Effects on `self` are substituted with the receiver place at the call.
- Inside the type: methods via `self.m()`, associated functions via `Self.g()` or `TypeName.g()`; an unqualified call `g()` to an associated function is `FN043`.
- **Dispatch** (T5): resolved at compile time from the **spelling** of the receiver's static type (after `let human: Human = hero`, `human.describe()` is `Human.describe`); for a type parameter, from the actual type per instance; for an anonymous object type, there are no methods; for a union receiver, only the universal built-in methods (`toString`) are callable without narrowing (`FN044`, `D42`).
- Generic methods may declare their own parameters after the name.
- Built-in types have methods (`Integer.parse`, `text.trim()`, collection methods): they are prelude declarations with the same rules.
- Lowering: JavaScript `T$m(self, …)` / `T$g(…)` module-level functions (no prototypes, no `this`), so structural values need no class; native: static functions, specialized on the receiver's view like any structural parameter.

```fa
-- ✓ me-1 → 24 / "13"
type Counter = {
	value: Integer
	format: (value: Integer) => String
	function create = (start: Integer): Counter => Counter { value = start, format = (value) => "{value}" }
	function double = (self): Integer => self.value * 2
	function increment = (self) mutates self.value => {
		self.value += 1
	}
	function display = (self) => console.log(self.format(self.value))
}
mutable counter = Counter.create(12)
console.log(counter.double())
counter.increment()
counter.display()

-- ✗ me-2: FN042
let o = { value = 1, function get = (self) => self.value }

-- ✗ me-3: FN043
type T = {
	function make = (): T => T {}
	function again = (): T => make()
}

-- ✗ me-4: FN041
type U = { function m = (self: U) => 1 }
```

### 4.10 Function expressions: alias, composition, partial application, factories

#### 4.10.1 Alias

`function b = a`: `b` is the same function: same signature, parameter names, defaults, generic parameters, chain. An alias of a generic function stays generic.

#### 4.10.2 Composition `a then b then c`

- Parsed as `Compose { operands }` (flat for chained `then`). Precedence: looser than `??`, tighter than `try`/`await`/`start`, `catch`.
- **Typing** (CF22): the first step may have any parameters (they are the composition's, with names and defaults); each following step has exactly one parameter without default (`FN090`); each step's parameter accepts the previous step's return type (`FN091` "`stringLength` expects a `String`, but receives an `Integer`"; a `Void`-returning step followed by another is `FN091`); the result is the last step's return type.
- **Effects**: `throws` = union of steps; `requires` = union; `mutates`/`invalidates` = the first step's (on its parameters) and every step's effects on module-level values and requirements; a following step that mutates its parameter: `FN092` "it receives a temporary value"; `async` if any step is. Pure iff every step is. Errors stop the chain (a step never receives an error).
- **Generic first step**: the composition is generic over the same parameters; at each call, the following steps' type parameters are inferred along the chain from the previous result (CF22). A step whose constraint isn't satisfied by the previous result: `GN010` at the call (middlewares in the wrong order).
- **Chains**: a `function` declared with `then` (or an alias of one, or a slice) keeps its **chain**, flat (`incrementAndDouble then square` has three steps). Slicing `f[1..]`, `f[1..3]`, `f[..=2]`, `f[1]`: only on a declaration with a known chain (`FN095` "`pipeline` is a plain `(Integer) => Integer`, it has no composition chain"); out of range `FN093`; a slice whose steps don't fit (impossible after a valid composition except for the first step of the slice taking one parameter: a slice starting at step `k > 0` takes that step's single parameter) — `FN094` if a slice's step types don't fit after generic re-inference. `f[1]` is an alias of the step.
- **Runtime operands**: `then` on function values known only at runtime (`let pipeline = parse then validator`) gives a plain function value (no chain); it captures its operands as an escaping closure would (copies).

#### 4.10.3 Partial application `f with (name = value, …)`

- Parsed as `PartialApplication { function, arguments }`; binds like a call.
- Binding by **name only** (`FN100` for a positional binding); unknown parameter `FN101`; bound twice or passed again later `FN102`; a mutated parameter `FN103`; the variadic parameter `FN104`.
- Bound values are evaluated **once**, when the `with` expression is evaluated, and **copied** into the new function (escaping-capture rules, `FN072` for move-only values); their own effects are effects of the expression that evaluates them.
- The result has the remaining parameters, in order, with their names and defaults, and the original effects. On a generic function, type parameters fixed by the bound values are fixed; the others stay generic (`D30`).
- `function xz = xyz with (y = 0)` needs compile-time bound values (§4.10.4); with runtime values, `let addTax = addPercent with (rate = readRate())` is a value.

#### 4.10.4 Factories known at compile time

`function add3 = makeAdd(3)`: the right side of a `function` declaration may be a call only if the callee is **pure** and every argument is a literal, a `static` value, or a function known at compile time (`FN110` otherwise: "`makeAdd(readNumber())` is only known at runtime: use `let add = makeAdd(readNumber())`"). The compile-time interpreter evaluates it to a closure value `(lambda, captured constants)`; the declaration gets the lambda's signature (names, defaults), and lowering emits the lambda body with the captured values as constants (one specialized function).

#### 4.10.5 Lowering

| Construct | JavaScript | Native |
| --- | --- | --- |
| alias | `const b = a` (or the same name at call sites) | same symbol (LLVM alias) |
| declared composition | `function f(...args) { return c(b(a(...args))) }`; call sites resolve named arguments against the first step | a generated function with direct calls (inlined) |
| runtime composition | `((f, g) => (...args) => g(f(...args)))(f, g)` | closure `{ fn, env = { f, g } }` |
| `with` (declaration) | `function xz(x, z) { return xyz(x, 0, z) }` | a specialized function, constant-propagated |
| `with` (value) | arrow function closing over `const` copies | closure |
| compile-time factory | the lambda with constants inlined | same |

#### 4.10.6 Tests

```fa
-- ✓ co-1 → 8 / 64
function increment = (x: Integer): Integer => x + 1
function double = (x: Integer): Integer => x * 2
function square = (x: Integer): Integer => x * x
function incrementAndDouble = increment then double
console.log(incrementAndDouble(3))
function all = incrementAndDouble then square
console.log(all(3))

-- ✗ co-2: FN091
function stringLength = (text: String): Integer => text.length
function invalid = stringLength then stringLength

-- ✓ co-3: errors accumulate, first error stops the chain → 0
type DatabaseError = Errors { Down }
function loadUser = (id: Integer): String throws DatabaseError => throw DatabaseError.Down
function loadScore = (user: String): Integer => 1
function loadUserScore = loadUser then loadScore
console.log(loadUserScore(42) catch error => 0)

-- ✓ co-4: slices → 9 / 1 / 6
function one = (x: Integer): Integer => x + 1
function two = (x: Integer): Integer => x * 2
function three = (x: Integer): Integer => x - 3
function four = (x: Integer): Integer => x // 4
function oneToFour = one then two then three then four
function twoToFour = oneToFour[1..]
function justTwo = oneToFour[1]
function oneToThree = oneToFour[..=2]
console.log(oneToThree(5))   -- ((5 + 1) * 2) - 3 = 9
console.log(twoToFour(4))    -- ((4 * 2) - 3) // 4 = 1  → prints 1
console.log(justTwo(3))      -- 6

-- ✗ co-5: FN095
function runFromTwo = (pipeline: (Integer) => Integer) => pipeline[1..](1)

-- ✗ co-6: FN093
function bad = oneToFour[7]

-- ✗ co-7: FN090
function add = (a: Integer, b: Integer) => a + b
function bad2 = one then add

-- ✗ co-8: FN092
type Box = { value: Integer }
function make = (v: Integer) => Box { value = v }
function bump = (box: Box) mutates box => {
	box.value += 1
}
function bad3 = make then bump

-- ✓ co-9: with → 3
function xyz = (x: Integer, y: Integer, z: Integer) => x + y + z
function xz = xyz with (y = 0)
console.log(xz(1, 2))

-- ✗ co-10: FN100 / FN102
function p1 = xyz with (0)
function p2 = xyz with (y = 0)
let r = p2(1, 2, y = 3)

-- ✓ co-11: factory known at compile time → 5
function makeAdd = (y: Integer) => (x: Integer) => x + y
function add3 = makeAdd(3)
console.log(add3(2))

-- ✗ co-12: FN110
function addN = makeAdd(readNumber())

-- ✓ co-13: runtime composition is a value → 4
let pipeline = increment then double
console.log(pipeline(1))

-- ✓ co-14: generic middleware chain → prints the timestamp field
type Request = { path: String }
function addTimestamp(Context: Request) = (request: Context): { ...Context, timestamp: Integer } => { ...request, timestamp = 1 }
function addAuth(Context: Request) = (request: Context): { ...Context, auth: String } => { ...request, auth = "t" }
function chain = addTimestamp then addAuth
console.log(chain(Request { path = "/" }).timestamp) -- 1

-- ✗ co-15: GN010 at the call (wrong order: logRequest needs `timestamp`)
function logRequest(Context: { path: String, timestamp: Integer }) = (request: Context): Context => request
function wrong = addAuth then logRequest then addTimestamp
let x = wrong(Request { path = "/" })
```

### 4.11 Overloading and what is left out

No overloading of functions, methods or operators: a second declaration with the same name in a scope is `TY009`; `+` on objects is `TY030` "no operator `+` for `Point`". No specialization, no higher-kinded types, no variance annotations, no implicit conversion to satisfy a constraint, no overload resolution on the return type.

---

## 5. Collections

The collections are written in Fa, in the standard library, on the internal `intrinsics` namespace for native (Q7), and as hand-written JavaScript classes or native JavaScript structures for JavaScript. The checker sees them as prelude declarations whose signatures carry the effects below: **no collection rule is special-cased in the checker** except list-literal typing, `[index]`/`[key]` proofs, the `T in c` reference identities (`getOrInsert` keeps the key identity, `add` returns fresh references) and fusion.

### 5.0 Common rules

#### 5.0.1 The family

| Type | Positions (`length`, indices) | Elements move on growth | Removal | Element constraint | Holds relation targets | Reference representation (native / JS) |
| --- | --- | --- | --- | --- | --- | --- |
| `Array(T)` | yes | yes | yes | — | no | pointer / object or index |
| `GrowingArray(T)` | yes | no | no | — | no | pointer / object or index |
| `OrderedColumnar(T)` | yes | yes | yes | object `T` | no | index / index |
| `Set(T)` | no (`size`) | yes | yes | `COMPARABLE`, exact element type | no | pointer / object or value |
| `GrowingSet(T)` | no | no | no | same | no | pointer / object or value |
| `Map(K, V)` | no | no (stable slots) | yes | `K` `COMPARABLE`, exact | no | pointer to value / object or key |
| `GrowingMap(K, V)` | no | no | no | same | no | same |
| `Pool(T)` | no | no | yes | — | yes | pointer / object or slot index |
| `GrowingPool(T)` | no | no | no | — | yes | pointer / object or index |
| `Columnar(T)` | no | no | yes | object `T` | yes | slot index / slot index |
| `GrowingColumnar(T)` | no | no | no | object `T` | yes | slot index / slot index |

- `Set`/`Map` key element types must be `COMPARABLE` (`CL020` "a `Set` element needs `==`: `Handler` contains a function"). `Columnar`/`OrderedColumnar` element types must be object types (`CL021`).
- Element types can't be reference types: `Array(Person in people)` is `CL022` "local collections of references are not allowed: keep indices, or declare a relation `Pool`" (R9). Relation pools (`Pool(Person in people)` as a field, `Pool(people->f)`) are relation sides, checked by the relations brief.
- `Void` elements: `TY071`.

#### 5.0.2 Creating collections

- A list literal `[…]` takes the collection type expected by the context (declaration annotation, parameter, return type, field type, element of another literal): `let numbers: Set(Integer) = [1, 2, 3]`. Elements are checked against the element type (literals adapt). A map literal's elements are 2-tuples `(key, value)` (`CL005`); duplicate **literal** keys in a map literal: `CL006` (`D61`); duplicates in a set literal are merged (`Set` semantics).
- Without context: `Array` with the joined element type (§3.1.2). An empty literal without context: `CL001` "an empty array needs its type".
- `C.from(source, options…)`: `source` is any collection (or list literal) whose elements satisfy the element type; the element type is inferred from the source or given (`Set(Float).from([1, 2, 3])`); always a copy. Options (named only): `capacity: Integer(min = 0)` for `Array`, `GrowingArray`, `Set`, `Map`, `Pool`, `Columnar`; `key: (value: T) => K` for `Set` and `GrowingSet` (`K` `COMPARABLE`, the function must be pure: `CL023`). Unknown option: `FN023`. `from` where the context already gives the type and no option is passed: `WCL001` (formatter rewrites into a literal).
- `Map.from([("foo", 12)])` infers `Map(String, Integer)` from tuples.

#### 5.0.3 `length` and `size`

`length` (positional collections and `String`) bounds the indices; `size` (the others) counts elements. Using the other one: `CL007` "a `Set` has a `size`, not a `length`". Both are read-only outside the collection's implementation (`CL008`).

#### 5.0.4 References into collections

- `c[i]` (proven), `c.get(i)`, `find`, `getOrInsert`, `add` (pools, columnars), loop variables and `values()` iterators give references of type `T in c` (optional for `get`/`find`). `use` keeps the reference, `let` copies (MR7). `use x = c.get(k)` has type `(T in c)?` and narrows with `is not none`.
- References to `Set`/`GrowingSet` elements and `Map`/`GrowingMap` keys are **read-only** even in a mutable collection (`CL031` "the elements of a set can't be modified: delete it and add the new value").
- Identities for "may designate" (owned by the reference checker): `c[literal]` exact; `c[param]` symbol; `getOrInsert(key)` the identity of `key`; `get(key)` exact when `key` is a literal; `add` fresh; `find` unknown.

#### 5.0.5 Iteration

| Iterating | Loop variables |
| --- | --- |
| `Array`, `GrowingArray`, `OrderedColumnar`, `Pool`, `GrowingPool`, `Columnar`, `GrowingColumnar` | references to the elements, mutable iff the collection is a mutable place |
| `Map`, `GrowingMap` with `(key, value)` | read-only key reference, value reference mutable iff the map is |
| `Set`, `GrowingSet` | read-only references |
| destructured `{ a, b }` | references to the fields (for a `Columnar`: reads only those columns) |
| `item, index` | only `Array`, `GrowingArray`, `OrderedColumnar` (`D38`) and `String`; elsewhere `CL032` |

While a loop (or an iterator) runs over `L`, the body can't mutate `L` or above, its `length`/`size`, or invalidate its elements (`EF031`, owned by the reference checker), including through calls and implied relation effects. Writing elements in place is allowed. Order: `Array`/`GrowingArray`/`OrderedColumnar`: index order; `Pool`/`Columnar`: slot order (no promise); `Set`/`Map`: unspecified.

#### 5.0.6 Sequence methods, fusion, iterators

Every collection has these (callbacks receive `(item, index)` on positional collections and `(item)` elsewhere; on maps, `(key, value)`, `D36`):

| Method | Result | Notes |
| --- | --- | --- |
| `includes(value)` | `Boolean` | `==` on elements (`COMPARABLE`, `CL020`); on maps, the keys |
| `find(predicate)` | `(T in self)?` | maps: `((K, V))?` copy (`D36`) |
| `filter(predicate)` | same collection kind (growing → standard counterpart), elements copied (`D34`) |
| `map(transform)` | `Array(R)`; on maps `Map(K, R)` (`D34`) |
| `reduce(combine, initial)` | `A` (generic accumulator, `combine` first, like JavaScript) |
| `every(p)`, `some(p)` | `Boolean` |
| `isEmpty()` | `Boolean` |
| `sum()` | the base numeric type (only for numeric elements, `CL024` otherwise; `D39`) |
| `values()` | `Iterator(T in self)` (maps: values; `keys()` for keys) |

- **Step by step** semantics (each method runs over the whole collection before the next, as JavaScript arrays). **Fusion**: a chain of these calls written as one expression is compiled into a single loop when **every callback is pure** (§4.4); otherwise it keeps intermediate collections. Fusion never changes results; it's decided on the typed tree (shared by both backends).
- **Iterators** (`values()`): lazy, element by element; methods `filter`, `map`, `find`, `reduce`, `every`, `some`, `firstValues(count: Integer(min = 0) = 1)` (an iterator), `toArray()`, and `for`. An iterator is a local reference into its collection (invalidated like a `use`); it can't be stored in a field, a collection, returned or captured by an escaping closure (`CL030`). A chain consumed where written is one loop on both targets.

#### 5.0.7 Structural variance

| Situation | Rule | Error |
| --- | --- | --- |
| Read-only parameter / `use` / loop, collection of supersets | `C(S)` viewed as `C(T)` when `S ≤ T` (element views; native specializes on stride and offsets, `D54` for scalar width/representation differences) | — |
| Mutated parameter (`mutates c`, `invalidates c[…]`) | exact element type on the written paths | `EF005` |
| `Set` elements, `Map` keys | exact (`S ≡ T`) even read-only | `CL010` "a `Set(Hero)` is never a `Set(Human)`" |
| `Map` values | general rule | — |
| Copy (`let a: Array(Human) = heroes`, `from`) | element-wise narrowing copy | — |
| Element added (`push`, `add`, `[i] = v`) | the value is copied with the element type's layout (literal: exact fields, `TY014`) | — |

#### 5.0.8 Equality

`==` on collections requires `COMPARABLE` elements. `Array`, `GrowingArray`, `OrderedColumnar`: same length, element-wise in order. `Set`: same size and every element of one is included in the other (by the right-hand set's key function when it has one, `D33`). `Map`: same keys, equal values. `Pool`, `Columnar`: equal as multisets (relation fields not compared). Different collection kinds can't be compared (`TY025`).

#### 5.0.9 Effects of every operation (normative table, from the reference)

| Collection | Operation | Effect |
| --- | --- | --- |
| `Array` | `array[index] = value` | `mutates self[index]` |
| | `delete(index)`, `take(index)` | `invalidates self[index..]` |
| | `pop` | `invalidates self[self.length - 1]` |
| | `push`, `insert`, `deleteWhere`, `clear`, `sortInPlace`, `reverseInPlace` | `mutates self` |
| `Map` | `map[key] = value` | `mutates self[key], self.size` |
| | `getOrInsert` | `mutates self.size` |
| | `delete(key)`, `take(key)` | `invalidates self[key]` |
| | `deleteWhere`, `clear` | `mutates self` |
| `Set` | `add`, `delete`, `take`, `deleteWhere`, `clear` | `mutates self` |
| `OrderedColumnar` | `add` | `mutates self` |
| | `[index] = value` | `mutates self[index]` |
| | `delete(index)`, `take(index)` | `invalidates self[index..]` |
| | `pop` | `invalidates self[self.length - 1]` |
| | `deleteWhere`, `clear` | `mutates self` |
| `Pool`, `Columnar` | `add` | `mutates self.size` (fresh reference) |
| | `delete(element)`, `take(element)` | `invalidates element` (implies `mutates self.size`) |
| | `deleteWhere`, `clear` | `mutates self` |
| `GrowingArray` | `push` | `mutates self.length` |
| | `array[index] = value` | `mutates self[index]` |
| `GrowingSet`, `GrowingMap`, `GrowingPool`, `GrowingColumnar` | `add` | `mutates self.size` (fresh for pools and columnars) |
| `GrowingMap` | `map[key] = value` | `mutates self[key], self.size` |
| all | reads (`[i]`, `get`, `includes`, `find`, `size`, `length`, sequence methods, iteration) | none |

Implied relation effects of `add`, `delete`, `take`, `deleteWhere`, `clear` are added by the relations machinery (`reference-checking#relations`).

#### 5.0.10 JavaScript in-place writes (`D60`)

A reference to an object or collection element is the JavaScript object itself; a later `place = newValue` (array element, map value, field, `use` alias) must therefore **write into the existing object** (`assign$T(target, source)`, field by field; for a collection, replace its contents) whenever a reference to that place may be live, so that references stay valid and see the new value as Fa specifies (`mutates self[index]` keeps references to the written item valid). When the reference checker proves no reference may designate the place, the backend replaces the object (cheaper). Places of union or optional type are replaced (references into their payload are invalidated anyway: owning edge).

### 5.1 `Array(T)`

#### API

| Member | Signature | Effect | Fails |
| --- | --- | --- | --- |
| `length` | `Integer(min = 0)` property | — | — |
| `a[i]` | `T in self` | — | `IndexError` unless `i` proven in `-length..length` (negative counts from the end) |
| `a[i] = v` | | `mutates self[i]` | same |
| `a[r]` (range) | `Array(T)` copy | — | `IndexError` unless both bounds are proven in `-length..=length` (`D40`); start after end: empty |
| `get(index: Integer)` | `(T in self)?` | — | never |
| `push(item: T)` | | `mutates self` | |
| `insert(index: Integer, item: T)` | | `mutates self` | `IndexError` unless `0 ≤ index ≤ length` proven (`D31`) |
| `delete(index: Integer)` | | `invalidates self[index..]` | `IndexError` unless proven |
| `take(index: Integer)` | `T` | `invalidates self[index..]` | `IndexError` unless proven |
| `pop()` | `T?` | `invalidates self[self.length - 1]` | |
| `clear()` | | `mutates self` | |
| `sortInPlace(compare = natural)` | `compare: (a: T, b: T) => Order`, `Order: Integer \| Float` (generic) | `mutates self` | natural order needs `ORDERED` (`CL025`) |
| `reverseInPlace()` | | `mutates self` | |
| `deleteWhere(predicate: (item: T) => Boolean)` | | `mutates self` | |
| `indexOf(value: T)` | `Integer(min = 0)?` | | |
| `sort(compare = natural)`, `reverse()` | `Array(T)` copy | | |
| sequence methods | §5.0.6 | | |
| `Array.from(source, capacity = 0)` | `Array(T)` | | |

Sorting is **stable** on every target, with the comparator's sign deciding (NaN counts as 0) (`D35`). `users.push(users[0])` is `EF030` (aliasing).

#### Index proofs (owned by range analysis)

Literal length facts (`[1, 2, 3]` until mutated), ranged index types, loop facts (`for index in 0..a.length`, `for item, index in a`), guards (`if index < a.length`, with `index >= 0` or a non-negative type), equal lengths after copy or `map`. A negative literal index `-1` is proven when `length ≥ 1` is known. Destructuring an array whose length isn't proven (`let [first, second] = values`) needs `try` (`IndexError`).

#### JavaScript

A JavaScript array (`capacity` ignored). Proven index: `a[i]`; unproven: a helper `faIndex(a, i)` that normalizes a negative index and throws `IndexError`; negative literal `a[a.length - 1]`. `push` → `push`; `insert` → `splice(i, 0, v)`; `delete` → `splice(i, 1)`; `take` → `splice(i, 1)[0]`; `pop` → `a.length ? a.pop() : null`; `clear` → `a.length = 0`; `sortInPlace` → `sort(cmp)` (natural order: a generated comparator, never the default string sort); `sort()` → `toSorted`; `reverse()` → `toReversed`; `deleteWhere` → in-place compaction loop (not `filter`, which allocates); slicing → `slice` + element copies when elements own memory; fused chains → plain `for` loops. References: object item → the object; primitive item → the index (`score *= 10` in a loop → `scores[i] *= 10`).

#### Native

`{ ptr, length, capacity }`; doubling growth from 4 items (or 64 bytes); `realloc`/`mremap`; inline `push` fast path; `memmove` for `insert`/`delete`; `deleteWhere` one-pass compaction with drops; proven index = plain load, unproven = one compare + cold error; slicing allocates + `memcpy` or `copy<T>`; reference = pointer.

#### Tests

```fa
-- ✓ ar-1 → ["Ana", "Dan", "Bao", "Cleo"] / ["Dan", "Bao", "Cleo"] / "Cleo" / []
mutable users = ["Ana", "Bao"]
users.push("Cleo")
users.insert(1, "Dan")
console.log(users)
users.delete(0)
console.log(users)
console.log(users.pop())
users.clear()
console.log(users)

-- ✓ ar-2: negative index and slices → 30 / [2, 3, 4] / [2, 3, 4] / [3, 4, 5] / [4, 5] / []
let numbers = [10, 20, 30]
console.log(numbers[-1])
let data = [1, 2, 3, 4, 5]
console.log(data[1..4])
console.log(data[1..=3])
console.log(data[2..])
console.log(data[-2..])
console.log(data[3..1])

-- ✗ ar-3: CL060 "the index is out of bounds: `array` has 3 items"
mutable array = [1, 2, 3]
let fourth = array[3]

-- ✗ ar-4: EF015 (unproven index needs try)
function readValue = (array: Array(Integer), index: Integer): Integer => array[index]

-- ✓ ar-5: get, ?? and catch → 0 / 0
let a = [1, 2]
console.log(a.get(5) ?? 0)
console.log(a[5] catch error => 0)

-- ✓ ar-6: loop variables are references → [10, 20, 30]
mutable scores = [1, 2, 3]
for score in scores {
	score *= 10
}
console.log(scores)

-- ✗ ar-7: EF031
for score in scores {
	scores.push(0)
}

-- ✗ ar-8: reference invalidated by push (reference checker error, EF032)
mutable list = ["Ana", "Bao", "Cleo"]
use first = list[0]
use last = list[2]
list.pop()
console.log(first)
list.push("Dan")
console.log(first)

-- ✗ ar-9: EF030
mutable people = ["a"]
people.push(people[0])

-- ✓ ar-10: sort copies, sortInPlace mutates → [1, 2, 3] / [3, 1, 2] / [3, 2, 1]
mutable ns = [3, 1, 2]
console.log(ns.sort())
console.log(ns)
ns.sortInPlace((a, b) => b - a)
console.log(ns)

-- ✓ ar-11: float comparator (D35) → [2.5, 1.5]
console.log([1.5, 2.5].sort((a, b) => b - a))

-- ✓ ar-12: deleteWhere → [3, 2]
mutable d = [3, 1, 2]
d.deleteWhere((n) => n < 2)
console.log(d)

-- ✓ ar-13: common methods → 3 / false / true / 1 / 3 / [3, 2] / [6, 2, 4] / 6 / true / true
let n = [3, 1, 2]
console.log(n.length)
console.log(n.isEmpty())
console.log(n.includes(2))
console.log(n.indexOf(1))
console.log(n.find((x) => x > 2))
console.log(n.filter((x) => x > 1))
console.log(n.map((x) => x * 2))
console.log(n.reduce((total, x) => total + x, 0))
console.log(n.every((x) => x > 0))
console.log(n.some((x) => x > 2))

-- ✓ ar-14: try let destructuring → 3
function firstTwo = (values: Array(Integer)): Integer throws IndexError => {
	try let [first, second] = values
	first + second
}
console.log(firstTwo([1, 2]) catch error => 0)

-- ✓ ar-15: fused chain (pure) gives the same result → 24
let total = [1, 2, 3, 4, 5, 6].filter((x) => x %% 2 == 0).map((x) => x * 2).reduce((s, x) => s + x, 0)
console.log(total)

-- ✓ ar-16: unfused chain keeps step-by-step order → "f1" "f2" "f3" "m2"
let r = [1, 2, 3].filter((x) => {
	console.log("f{x}")
	x == 2
}).map((x) => {
	console.log("m{x}")
	x
})

-- ✓ ar-17: iterator is element by element → "f1" "f2" "m2" "f3"
let r2 = [1, 2, 3].values().filter((x) => {
	console.log("f{x}")
	x == 2
}).map((x) => {
	console.log("m{x}")
	x
}).toArray()

-- ✓ ar-18: firstValues → [2, 4]
console.log([1, 2, 3, 4, 5, 6].values().filter((x) => x %% 2 == 0).firstValues(2).toArray())

-- ✗ ar-19: CL001
let empty = []

-- ✓ ar-20: WCL001 (warning)
let xs: Array(Integer) = Array.from([1, 2])

-- ✓ ar-21: superset collection to a read-only parameter → "John"
type Human = { name: String }
type SuperHuman = { ...Human, powers: String }
function logNames = (humans: Array(Human)) => {
	for human in humans {
		console.log(human.name)
	}
}
logNames([SuperHuman { name = "John", powers = "x" }])

-- ✗ ar-22: EF005
function addJohn = (humans: Array(Human)) mutates humans => {
	humans.push({ name = "John" })
}
mutable supers = [SuperHuman { name = "A", powers = "x" }]
addJohn(supers)

-- ✓ ar-23: the generic form → 0
function keepNamed(Item: Human) = (items: Array(Item)) mutates items => {
	items.deleteWhere((item) => item.name == "")
}
mutable s2 = [SuperHuman { name = "", powers = "x" }]
keepNamed(s2)
console.log(s2.length)

-- ✓ ar-24: D60 (JS in-place write keeps the reference) → "Bao"
type P = { name: String }
mutable ps = [P { name = "Ana" }]
use first = ps[0]
ps[0] = P { name = "Bao" }
console.log(first.name)

-- ✗ ar-25: TY071
let voids = [1].map((x) => console.log(x))
```

### 5.2 `GrowingArray(T)` (and the growing family)

Same API as `Array` **minus** `delete`, `take`, `deleteWhere`, `pop`, `clear`, `insert`, `sortInPlace`, `reverseInPlace` (calling one: `CL040` "a `GrowingArray` never removes or moves its items"). `push` declares `mutates self.length` (invalidates nothing); `[i] = v` `mutates self[i]`. References are valid for the collection's whole life.

- JavaScript: a JavaScript array.
- Native: segmented list (first segment `2^k` items, ~64 bytes; segment `s = 0` if `i < 2^k`, else `floor(log2(i >> k)) + 1`; offset `i - 2^(k+s-1)`; up to 64 segment pointers in the header). Iteration: a loop per segment.

```fa
-- ✓ ga-1: references survive push → "Ana"
type Human = { name: String, age: Integer }
mutable growing: GrowingArray(Human) = [{ name = "Ana", age = 30 }]
use firstGrowing = growing[0]
growing.push({ name = "Bao", age = 25 })
console.log(firstGrowing.name)

-- ✗ ga-2: CL040
growing.pop()

-- ✗ ga-3: EF032 (Array.push invalidates, unlike GrowingArray.push)
mutable array: Array(Human) = [{ name = "Ana", age = 30 }]
use first = array[0]
array.push({ name = "Bao", age = 25 })
console.log(first.name)
```

### 5.3 `Map(K, V)` and `GrowingMap(K, V)`

#### API

| Member | Signature | Effect | Fails |
| --- | --- | --- | --- |
| `size` | `Integer(min = 0)` | | |
| `m[key]` | `V in self` | | `KeyNotFound` unless presence proven (`D32`) |
| `m[key] = value` | | `mutates self[key], self.size` | never (adds or replaces) |
| `m[key] op= value` | | `mutates self[key]` | `KeyNotFound` unless proven (`try counts[key] += 1`) |
| `get(key: K)` | `(V in self)?` | | |
| `getOrInsert(key: K, value: V)` | `V in self` (identity of `key`) | `mutates self.size` | |
| `includes(key: K)` | `Boolean` | | |
| `delete(key: K)` | `Boolean` (whether it was present, `D37`) | `invalidates self[key]` | |
| `take(key: K)` | `V?` (`D37`) | `invalidates self[key]` | |
| `deleteWhere(predicate: (key: K, value: V) => Boolean)` | | `mutates self` | |
| `clear()` | | `mutates self` | |
| `keys()` | `Iterator(K in self)` (read-only) | | |
| `values()` | `Iterator(V in self)` | | |
| iteration `for (key, value) in m` | read-only key, value mutable iff `m` is | | |
| `Map.from(source)` | | | |

`GrowingMap`: no `delete`, `take`, `deleteWhere`, `clear` (`CL040`). Writing a key while iterating the map is `EF031` (it may change the size): write through the loop's value instead. Keys are compared with `==` (deep), floats with SameValueZero.

**Presence proofs** (`D32`, fixed rules, target-independent): `m[k]` needs no `try` when (1) `m` was created from a literal with literal keys, is unchanged since, and `k` is a literal among them; (2) inside `if m.includes(k)` (or after `if not m.includes(k) { return }`), where `k` is an immutable binding or a literal, until `m` is mutated; (3) `k` has the identity of a reference obtained from `getOrInsert(k, …)` since the last mutation. Nothing else.

#### JavaScript

Primitive keys (numbers, bigints, strings, booleans, enums, fields, literal unions): a JavaScript `Map` (insertion order, allowed). Object or tuple keys: `FaHashMap`, a JavaScript `Map` from the generated 32-bit structural hash to an entry `{ key, value }` or an array of entries, `==` by the generated `eq`. Both expose one internal interface (`get`, `set`, `has`, `delete`, `size`, iteration), so a function taking a `Map(K, V)` doesn't depend on which one it got (the kind is fixed by `K`). References: object value → the object; primitive value → the key (`use count = counts.getOrInsert(word, 0); count += 1` → `counts.set(word, counts.get(word) + 1)`, after the `getOrInsert`).

#### Native

Compact dict: entries `{ hash: u32, key, value }` in a segmented, never-moving arena with per-chunk free lists and occupancy bitmaps; a Swiss-table index of `{ control: u8, slot: u32 }` (7/8 load, SIMD groups, tombstones until rebuild, rebuild from stored hashes); maps of ≤ 8 entries have no index; per-map random seed; iteration over the entries. `GrowingMap`: no free lists, no bitmap, no tombstones.

#### Tests

```fa
-- ✓ ma-1 → 0 / 12 / 0
let scores: Map(String, Integer) = [("ana", 12), ("bao", 17)]
console.log(scores["cleo"] catch error => 0)
console.log(scores["ana"])       -- proven: literal map, literal key (D32)
console.log(scores.get("cleo") ?? 0)

-- ✗ ma-2: EF015 (KeyNotFound)
function scoreOf = (scores: Map(String, Integer), name: String): Integer => scores[name]

-- ✓ ma-3: presence proven by a guard (D32) → 12
function scoreIf = (scores: Map(String, Integer), name: String): Integer => {
	if scores.includes(name) {
		return scores[name]
	}
	0
}
console.log(scoreIf(scores, "ana"))

-- ✓ ma-4: add, delete, size → true / 1 / false / 0
mutable inventory: Map(String, Integer) = []
inventory["apples"] = 3
inventory["oranges"] = 2
inventory.delete("apples")
console.log(inventory.includes("oranges"))
console.log(inventory.size)
console.log(inventory.isEmpty())
inventory.clear()
console.log(inventory.size)

-- ✓ ma-5: getOrInsert with use → 2
mutable counts: Map(String, Integer) = []
for word in ["a", "b", "a"] {
	use count = counts.getOrInsert(word, 0)
	count += 1
}
console.log(counts["a"] catch error => 0)

-- ✓ ma-6: stable references across insertions and other deletions → 10 / 10
mutable sc: Map(String, Integer) = [("ana", 10), ("bao", 7)]
use score = sc.get("ana")
sc["cleo"] = 7
sc.delete("bao")
if score is not none {
	console.log(score)
}
console.log(sc["ana"] catch error => 0)

-- ✗ ma-7: EF032 (delete of an unknown key invalidates `score`)
let loser = readName()
sc.delete(loser)
console.log(score)

-- ✗ ma-8: EF031 (writing a key while iterating)
for (name, value) in sc {
	sc[name] = value + 1
}

-- ✓ ma-9: writing through the loop's value → 11
for (name, value) in sc {
	value += 1
}
console.log(sc["ana"] catch error => 0)

-- ✗ ma-10: CL031 (keys are read-only)
for (name, value) in sc {
	name = "x"
}

-- ✓ ma-11: tuple keys → "origin"
let byPoint: Map(Tuple(Integer, Integer), String) = [((0, 0), "origin")]
console.log(byPoint.get((0, 0)) ?? "")

-- ✗ ma-12: CL010 (Map keys are exact)
type Human = { name: String }
type Hero = { name: String, power: String }
function f = (m: Map(Human, Integer)) => m.size
let heroes: Map(Hero, Integer) = []
f(heroes)

-- ✗ ma-13: CL006
let dup: Map(String, Integer) = [("a", 1), ("a", 2)]

-- ✗ ma-14: CL040
mutable g: GrowingMap(String, Integer) = []
g.delete("a")

-- ✓ ma-15: map over values keeps keys (D34) → 2
let doubled = scores.map((key, value) => value * 2)
console.log(doubled.size)
```

### 5.4 `Set(T)` and `GrowingSet(T)`

#### API

| Member | Signature | Effect |
| --- | --- | --- |
| `size`, `isEmpty()`, `includes(value: T)` | | |
| `add(value: T)` | `Boolean` (inserted) | `mutates self` (`GrowingSet`: `mutates self.size`) |
| `delete(value: T)` | `Boolean` (was present, `D37`) | `mutates self` |
| `take(value: T)` | `T?` | `mutates self` |
| `deleteWhere(predicate)`, `clear()` | | `mutates self` |
| `find(predicate)` | `(T in self)?` read-only | |
| `union(other: Set(T))`, `intersection`, `difference` | `Set(T)` | |
| `isSubsetOf(other: Set(T))` | `Boolean` | |
| `Set.from(source, key = …, capacity = …)` | | |

`GrowingSet` lacks `delete`, `take`, `deleteWhere`, `clear` (`CL040`). Elements are read-only (`CL031`); element types exact (`CL010`). With `key`, two values are the same when their keys are `==`; only the key is hashed. The `key` function is part of the set **value**, not its type (`D33`): set operations between sets with different key functions use the receiver's.

#### JavaScript

Primitive elements without `key`: a JavaScript `Set`. Otherwise `FaSet` (hash-bucketed by the generated hash, or a JavaScript `Map` from a primitive key to the value), which **implements the JavaScript `Set` interface** (`has`, `add`, `delete`, `size`, iteration) so that code taking a `Set(T)` is the same whatever the instance (`D33`). `add` returning a `Boolean` compiles to `(s.size !== (s.add(v), s.size))`. Set operations are generated loops. References: the element (objects) or the value (primitives, read-only anyway).

#### Native

Swiss table with inline values (control bytes + values in one allocation, mirrored first group, triangular probing, 7/8 load, same-size rebuild to purge tombstones); `hash`/`==` inlined by monomorphization; a `key` function known at the construction site is inlined, otherwise called through the `{ fn, env }` stored in the header. `GrowingSet`: values in stable pages plus an index of slot numbers.

#### Tests

```fa
-- ✓ se-1 → 3 / 3
let setOfIntegers: Set(Integer) = [1, 2, 2, 3]
console.log(setOfIntegers.size)
console.log(Set.from([1, 2, 2, 3]).size)

-- ✓ se-2: deep equality decides uniqueness → 3 / false
type Human = { name: String, age: Integer }
mutable humans: Set(Human) = [{ name = "John", age = 20 }, { name = "Jane", age = 21 }, { name = "John", age = 22 }]
console.log(humans.size)
console.log(humans.add(Human { name = "John", age = 20 }))

-- ✓ se-3: key option → 2
type User = { id: Integer, email: String }
mutable users = Set.from([User { id = 1, email = "a" }, User { id = 2, email = "b" }], key = (user) => user.id)
users.add({ id = 1, email = "c" })
console.log(users.size)

-- ✓ se-4: set operations → [1, 2, 3, 4] / [2, 3] / [1] / false / true
let a: Set(Integer) = [1, 2, 3]
let b: Set(Integer) = [2, 3, 4]
console.log(a.union(b))
console.log(a.intersection(b))
console.log(a.difference(b))
console.log(a.isSubsetOf(b))
let c: Set(Integer) = [3, 2, 1]
console.log(a == c)

-- ✗ se-5: EF032 (add invalidates every reference into the set)
type Person = { name: String }
mutable people: Set(Person) = [{ name = "Ana" }]
use ana = people.find((person) => person.name == "Ana")
people.add({ name = "Bao" })
console.log(ana?.name)

-- ✗ se-6: CL031
mutable tags: Set(String) = ["fa"]
for tag in tags {
	tag = "x"
}

-- ✗ se-7: CL020
type Handler = { run: () => Void }
let hs: Set(Handler) = []

-- ✓ se-8: key set as a set of literals → 2
let myKeySet: Set("hello" | "world" | "!") = ["hello", "!"]
console.log(myKeySet.size)

-- ✓ se-9: NaN and -0 under SameValueZero → 2
let fs: Set(Float) = [0.0, -0.0, 0.0 / 0.0, 0.0 / 0.0]
console.log(fs.size)

-- ✓ se-10: take → "fa" / none
mutable t: Set(String) = ["fa"]
console.log(t.take("fa"))
console.log(t.take("fa"))
```

### 5.5 `Pool(T)` and `GrowingPool(T)`

#### API

| Member | Signature | Effect |
| --- | --- | --- |
| `size`, `isEmpty()` | | |
| `add(value: T)` | `T in self`, fresh | `mutates self.size` |
| `delete(element: T in self)` | | `invalidates element` (+ implied relation effects) |
| `take(element: T in self)` | `T` (a copy without links) | `invalidates element` |
| `deleteWhere(predicate)`, `clear()` | | `mutates self` |
| `find(predicate)` | `(T in self)?` | |
| `includes(value: T)` | `Boolean` (`==`) | |
| sequence methods | §5.0.6 (`filter` → `Pool(T)`, `map` → `Array(R)`) | |
| `Pool.from(source)` | | |

- `delete`/`take` require an argument typed `T in p` with `p` the **same** pool place (`CL050` "`ana` is not an element of `pool`": a reference into another pool, or a copy); deleting during iteration over the pool: `EF031` (use `deleteWhere`).
- `add({ … })`: an object literal is built in place; it must have exactly the element's fields (`TY014`). An existing value with more fields is copied without them. `let p = pool.add(…)` copies (allowed, `WCL002` "bad practice: `p` is a copy, bind the reference with `use`").
- `GrowingPool`: no `delete`, `take`, `deleteWhere`, `clear` (`CL040`); `add` `mutates self.size`.

#### JavaScript

Object elements: a JavaScript `Set` of the objects (identity semantics are right: two equal humans are two elements); `add` → `set.add(obj)` and returns `obj` (a literal is created once; an existing value is moved when dead and of the exact shape, else copied by the element's copy function); `delete` → `set.delete(ref)` (plus relation unlinking); iteration → `for…of`. Primitive elements: slot array + occupancy array + free-list stack; a reference is the slot index. `GrowingPool`: a JavaScript array (object or index references).

#### Native

Slab: size-aligned chunks (64 KiB or ≥ 64 slots), chunk found by masking the pointer, per-chunk occupancy bitmap and intrusive free list, bitset of chunks with free slots (lowest first), one spare chunk kept, no compaction, `tzcnt` iteration, in-place construction after evaluating failing/calling field expressions first, destruction without visiting flat elements. `GrowingPool`: bump allocation in the last chunk.

#### Tests

```fa
-- ✓ po-1: stable references → 21
type Human = { name: String, age: Integer }
mutable pool: Pool(Human) = []
use john = pool.add({ name = "John", age = 20 })
pool.add({ name = "Alice", age = 25 })
pool.add({ name = "Bob", age = 32 })
john.age += 1
console.log(john.age)

-- ✓ po-2: delete invalidates only the deleted element → "Jane"
use jane = pool.add({ name = "Jane", age = 21 })
pool.delete(john)
console.log(jane.name)

-- ✗ po-3: EF032
console.log(john.name)

-- ✗ po-4: EF032 (`someone` may be `jane`)
use someone = pool.find((human) => human.age > 30)
if someone is not none {
	pool.delete(someone)
	console.log(jane.name)
}

-- ✓ po-5: a reference found before an add stays distinct from it (fresh) → "Bao"
mutable p2: Pool(Human) = []
p2.add(Human { name = "Ana", age = 1 })
use found = p2.find((person) => person.name == "Ana")
use added = p2.add(Human { name = "Bao", age = 2 })
if found is not none {
	p2.delete(found)
	console.log(added.name)
}

-- ✗ po-6: EF031
for human in pool {
	pool.delete(human)
}

-- ✓ po-7: deleteWhere (the pool holds Alice, Bob and Jane here) → 2
pool.deleteWhere((human) => human.name == "Jane")
console.log(pool.size)

-- ✗ po-8: CL050
mutable other: Pool(Human) = []
use x = other.add({ name = "X", age = 0 })
pool.delete(x)

-- ✗ po-9: TY014 (literal with an extra field)
type Point = { x: Integer, y: Integer }
mutable points: Pool(Point) = []
points.add({ x = 1, y = 2, z = 3 })

-- ✓ po-10: WCL002 (let copies) → 3 / 4
use point = points.add({ x = 3, y = 4 })
let copy = points.add({ x = 3, y = 4 })
point.x += 1
console.log(copy.x)
console.log(point.x)

-- ✓ po-11: two equal values are two elements → 2
mutable twins: Pool(Human) = []
twins.add({ name = "A", age = 1 })
twins.add({ name = "A", age = 1 })
console.log(twins.size)

-- ✗ po-12: EF030 (the callback captures a reference into the mutated pool)
mutable ps: Pool(Human) = []
use ana = ps.add(Human { name = "Ana", age = 31 })
ps.deleteWhere((person) => person.age < ana.age)
```

### 5.6 `Columnar(T)`, `OrderedColumnar(T)`, `GrowingColumnar(T)`

#### API

- `Columnar(T)`: the `Pool` API (§5.5): `add` (fresh, `mutates self.size`), `delete(element)`/`take(element)` (`invalidates element`), `deleteWhere`, `clear` (`mutates self`), `size`, `find`, `includes`, iteration. A reference is a slot index; copying an element out is a gather.
- `OrderedColumnar(T)`: the positional `Array` API (MR6): `add(value)` appends and returns `T in self` (`mutates self`), `length`, `c[i]`, `c[i] = v` (`mutates self[i]`), `get(i)`, `delete(i)`/`take(i)` (`invalidates self[index..]`), `pop()` (`invalidates self[self.length - 1]`), `deleteWhere`, `clear`, `item, index` iteration. No growing counterpart; can't hold relation targets (`CL051`).
- `GrowingColumnar(T)`: `add` only (`mutates self.size`, fresh).
- `for { a, b } in columnar` reads only the named columns; `for item in columnar` gathers each element.

#### JavaScript

One array per field: `Float64Array` for `Float` and default `Integer`, `Int32Array`/`Uint8Array` when the integer range fits, `Uint8Array` for `Boolean`, a plain `Array` otherwise; all with the same capacity, grown together (`set`); occupancy `Uint32Array` bitmap and an `Int32Array` free-slot stack. A reference is the slot index (`john.age += 1` → `ages[john] += 1`). `for { age } in c` → a loop over slots testing the bitmap; `for human in c` builds an object per element. `OrderedColumnar`: no bitmap; `delete(i)` shifts each column with `copyWithin`.

#### Native

One column per field with the field's layout; chunk capacity `2^k` slots so the widest column fills ~64 KiB; one allocation per chunk holding every column run (64-byte aligned) after a header (bitmap, count, free-list head); slot `i` → chunk `i >> k`, offset `i & mask`; the free list lives in the first column's dead slot (≥ 4 bytes); single-field loops vectorize with the occupancy word as a mask. `OrderedColumnar`: one `Array` per field grown together; references are indices. `GrowingColumnar`: bump index.

#### Tests

```fa
-- ✓ cm-1 → "John is 20" "Jane is 21" (slot order) / 41
type Human = { name: String, age: Integer }
mutable columnar: Columnar(Human) = []
columnar.add({ name = "John", age = 20 })
columnar.add({ name = "Jane", age = 21 })
for { name, age } in columnar {
	console.log("{name} is {age}")
}
mutable total = 0
for { age } in columnar {
	total += age
}
console.log(total)

-- ✓ cm-2: references → 31 / "Jane"
use john = columnar.add({ name = "John", age = 30 })
use jane = columnar.add({ name = "Jane", age = 28 })
john.age += 1
console.log(john.age)
columnar.delete(john)
console.log(jane.name)

-- ✗ cm-3: EF032
console.log(john.name)

-- ✓ cm-4: ordered columnar is positional → ["John", "John"]
mutable ordered: OrderedColumnar(Human) = []
ordered.add({ name = "John", age = 20 })
ordered.add({ name = "Jane", age = 21 })
ordered.add({ name = "John", age = 22 })
ordered.delete(1)
console.log(ordered.map((h) => h.name))

-- ✗ cm-5: EF032 (OrderedColumnar.add declares `mutates self`)
use first = ordered[0]
ordered.add({ name = "X", age = 1 })
console.log(first.name)

-- ✗ cm-6: CL021
let c: Columnar(Integer) = []

-- ✗ cm-7: CL032 (Columnar has no positions)
for human, index in columnar { }
```

### 5.7 Native and JavaScript runtime pieces to build

| Piece | Native (Fa on `intrinsics`) | JavaScript (hand-written runtime module) |
| --- | --- | --- |
| `Array` | `Vec`-like + growth function | JavaScript array + `faIndex`, `faSlice` helpers |
| `GrowingArray` | segmented list | JavaScript array |
| `Set` | Swiss table | `Set` / `FaSet` (Set interface) |
| `GrowingSet` | paged values + index | same as `Set` |
| `Map` | compact dict | `Map` / `FaHashMap` (Map interface) |
| `GrowingMap` | compact dict without deletion | same as `Map` |
| `Pool` | slab | `Set` (objects) / `FaSlotPool` (primitives) |
| `GrowingPool` | chunk bump list | JavaScript array |
| `Columnar` | chunked columns | `FaColumnar` (typed arrays) |
| `OrderedColumnar` | one `Array` per field | `FaOrderedColumnar` |
| `GrowingColumnar` | chunked columns, bump | `FaColumnar` without free list |
| `Iterator` | struct (position + callbacks), or fused loop | fused loop, or a generator-free iterator object |
| per type | `hash`, `eq`, `copy`, `drop`, `format` generated | `hash$T`, `eq$T`, `copy$T`, `assign$T`, `format$T` generated |

---

## Diagnostics index

One line per code; the section that defines the condition is in parentheses. Reference-checker and range-analysis codes used here (`EF030`–`EF032`, `CL060`) are owned by the sibling brief.

- **Types.** `TY001` mutable type · `TY002` value doesn't satisfy the type · `TY003` generic type without its parameters · `TY005` object built by a call · `TY006` mixed integer/float literals in a join · `TY007` unknown type (and `Any`) · `TY008` value/type confusion · `TY009` duplicate declaration (no overloading) · `TY010`–`TY012` duplicate field / field-method clash / duplicate method · `TY013` default refers to a field · `TY014` extra field in a literal, `.=` or spread · `TY015` missing field · `TY016` private access · `TY017` field given twice · `TY018` replaced field of another type · `TY019` unknown field · `TY020` field of an optional without `?.` · `TY021` field of an unnarrowed named union · `TY022` readonly write · `TY023` write through an immutable place · `TY024` method assigned · `TY025` `==` on a non-comparable type · `TY026` existing value missing a field · `TY030` no operator for this type (§4.11) · `TY040` indistinguishable union members · `TY041` value fits several members · `TY042`/`WTY002` static `is` never/always true · `TY043` `is` with a non-member type · `TY044` `?.`/`??` on a non-optional · `TY050`–`TY057` named unions (§3.5) · `TY060` type contains itself · `TY070` `Void` stored · `TY071` `Void` in a data position · `TY072` function type without `=> Void` · `TY073` reference type outside the allowed positions · `TY080`–`TY085` enumerations (§3.8) · `TY090`–`TY092` `Fields` (§3.9) · `TY100`/`TY101` spread of a non-object / conflicting field · `TY110`/`TY111` difference on an unsplittable type / of a missing member · `WTY001` type written twice.
- **Generics.** `GN001`–`GN008` declaration and arity (§3.11.9) · `GN010` constraint not satisfied · `GN011` several union members fit · `GN012` difference of a type parameter in a generic signature · `GN013` missing required method · `GN014` two inferred types · `GN015` not inferable · `GN016` operation not allowed by the constraint · `GN017` error after substitution · `GN020` polymorphic recursion · `GN030` generic lambda.
- **Type functions and reflection.** `TF001` nesting over 256 · `TF002` error inside an applied type function · `TF003` compile-time step limit · `TF004` compile-time call threw · `TF005` type/value function confusion · `TF006` non-constant condition · `TF007` no `when` branch matches · `TF010` unknown key in a key set · `TF011` error in an unrolled `@for` body · `TF020`–`TF028` built-in type function misuse (§3.12.4) · `TF040`–`TF043` invalid schema.
- **Conversions.** `CV001` mixing kinds of numbers · `CV002` no `String(x)` · `CV003` `as` outside destructuring and `when` · `CV005` view between `number` and `bigint` integer storages · `WCV001` identity conversion.
- **Functions.** `FN001` lambda as a declaration value · `FN010` recursion without a return type · `FN011` missing value / bare `return` · `FN012` `Never` function can return · `FN013` `return value` without a return type · `FN014` `return` outside a function · `FN015` local functions calling each other · `FN016` `await` outside an async context · `FN020` default uses a later parameter · `FN021`–`FN029` argument binding (§4.2.2) · `FN040`–`FN044` `self`, methods, dispatch (§4.9) · `FN050` compile-time call of an impure function · `FN060` function doesn't declare an effect of the stored function · `FN061` unannotated lambda parameter without context · `FN062` lambda with too many parameters · `FN063` async lambda without an async expected type · `FN070` write to a value captured by copy · `FN071` captured `mutable use` · `FN072` move-only value captured by copy or bound by `with` · `FN080`–`FN086` function-type fit (§4.8.2) · `FN090`–`FN095` composition (§4.10.2) · `FN100`–`FN104` `with` (§4.10.3) · `FN110` factory not known at compile time · `WFN001` dropped pure value · `WFN002` destructuring with an inline anonymous type.
- **Effects.** `EF001`/`EF002`/`EF003`/`EF010` undeclared `mutates`/`invalidates`/`requires`/`throws` in a block body · `EF004` write-set not exact · `EF005` mutated collection of supersets · `EF011` mutated argument not a mutable place · `EF012` temporary given to a mutated parameter · `EF013` parameter replaced · `EF014` primitive parameter mutated · `EF015` failing call without `try`/`catch` · `EF016` unsatisfied requirement · `EF017` `try` and `catch` together · `EF018` `catch` of a non-stored callback's errors · `EF020` invalid type after `throws` · `EF021` `throw` of a non-error · `EF030` overlapping arguments · `EF031` structural change while iterating · `EF032` use of an invalidated reference · `EF040`–`EF044` effect paths (§4.3.3) · `WEF001` declared effect absent · `WEF002` `try` on an expression that can't fail.
- **Collections.** `CL001` empty literal without type · `CL005` map literal element not a pair · `CL006` duplicate literal map keys · `CL007` `length`/`size` confusion · `CL008` `length`/`size` written · `CL010` `Set` element / `Map` key types are exact · `CL020` element or key not comparable · `CL021` columnar of a non-object type · `CL022` local collection of references · `CL023` impure `key` function · `CL024` `sum` on non-numbers · `CL025` natural order on non-ordered elements · `CL030` iterator stored, returned or captured · `CL031` write to a set element or a map key · `CL032` `item, index` on a collection without positions · `CL040` removal method on a growing collection · `CL050` element of another collection given to `delete`/`take` · `CL051` ordered columnar holding relation targets · `CL060` index proven out of bounds · `WCL001` `from` where a literal suffices · `WCL002` `let` copy of an `add` result.

---

## Ambiguities and decisions

Each item: what is ambiguous or contradictory (with the pages), then the decision applied in this brief. Criteria, in order: one unambiguous way to write each thing; no surprise for JavaScript developers; cheap JavaScript without boxing primitives; fine-grained effects; consistency with the decisions already recorded in `design/spec-review.md` and `design/compilation-with-llvm.md`.

**D1. Absorption of object members in union normalization.** `unions.md` says "a member that another member already covers disappears", and also that `{ name: String } | { name: String, age: Integer }` is an error. Structurally the second member is covered by the first, so absorption would silently make it `{ name: String }`. **Decision:** absorption applies to literals, ranges, `None`, enum and named-union members, `Fields` and collections only; object, tuple and function members are never absorbed, and the distinguishability rule decides (`TY040`).

**D2. `Integer` versus `Int64`.** `numbers.md` says "`Integer` is an alias for `Int64`", while the JavaScript rule says `Int64` *written explicitly* compiles to a `bigint` and `Integer` to a `number` with ±(2^53 − 1) checks. Two aliases can't have two representations. **Decision:** `Integer` and `Int64` are two integer types with the same range and the same native storage, differing by a storage tag (`Default` vs `I64`). Values convert implicitly in both directions (ranges fit; on JavaScript `Int64 → Integer` is `Number(b)` with the 2^53 stop, the target-dependent stop JavaScript already has). **Views** inside aggregates (elements of a collection parameter, fields reached by reference) require the same JavaScript class (`number` vs `bigint`), else `CV005` with the `map` fix. Reword `numbers.md` "alias" into "has the range and native storage of".

**D3. Nominal types and structural constraints.** A type with `#private` fields "is distinct from every other type"; can it still be passed where `{ label: String }` or a generic constraint `{ describe: (self) => String }` is expected? **Decision:** yes. Nominality only affects identity (`S ≤ Nominal` iff `S ≡ Nominal`); a nominal type satisfies structural object types through its public data fields and methods. Otherwise encapsulated types couldn't use structural polymorphism, which is Fa's only polymorphism.

**D4. Named-union members and their payload type.** "A `Shape.Circle` … its fields can be read like those of `{ radius: Float }`", but can it be *passed* as one, and what about `UserId.User` and `Integer`? **Decision:** a member with an **object** payload satisfies its payload object type (reads, views, structural parameters); a member with a non-object payload does not (bind it with `as`), which keeps the single-member-union idiom for distinct ids sound. A payload given as a *named* object type (`Car = CarData`) counts as an object payload, but is built with parentheses (`U.Car(value)`), like any non-literal type definition.

**D5. Writes through a type parameter or `Type(parameter)`.** `structural-typing.md#keeping-the-type-of-the-argument` copies `human` (`Type(human)`) and writes `copy.name = name`; with an argument whose `name` is `"a" | "b"`, this writes any `String` into a narrower field. **Decision:** writes to fields reached through a value of a type parameter (explicit or implicit) are recorded in the function's write-set in terms of the constraint's fields, and every call checks the actual type for exactness on those leaves (`EF004`), exactly like "mutating through a narrower type". The documented examples stay valid.

**D6. Field access on anonymous unions.** Not specified. **Decision:** reading a field common to every object member is allowed (type: union of the member field types), TypeScript-style; writing it is allowed only if the field type is identical in every member. Needed to read a discriminant (`shape.kind`).

**D7. Runtime kinds for distinguishability.** `unions.md` lists number / `BigInt` / string / boolean / `none` / function / collection / object, but enumerations, `Fields`, tuples, the explicit 64-bit integers and the different collections are not placed. **Decision:** the kinds of §3.4.2, chosen so that every runtime test is one JavaScript `typeof`/`instanceof`/`Array.isArray`: enumerations and ≤ 32-flag `Fields` are `number`; `Int64`/`UInt64`/128-bit and > 32-flag `Fields` are `bigint`; tuples share the `array` kind with `Array`, `GrowingArray`, `OrderedColumnar`; each other collection constructor is its own kind.

**D8. Several named unions in one anonymous union** (`ApiError | StorageError`, the most common `throws` and `catch` type). Tags as "declaration index" (native) collide between unions. **Decision:** different named unions are always distinguishable: native wraps them in the anonymous union's own tag; JavaScript uses a **global** tag id per member (`$t`), assigned per program.

**D9. Spreading a superset value into an object literal.** `objects.md`: "the literal still follows the rule of object literals". **Decision:** the spread source can't bring fields the expected type lacks (`TY014`); narrowing is done by a copy to an annotated binding, not by a literal.

**D10. Representation of `Shape - Shape.Point` and of narrowed subsets.** **Decision:** a member mask on the nominal union (`NamedUnion(n, mask)`), so the difference "keeps the identity" and a narrowed value keeps the layout and tags of the full union; an anonymous union of members of one named union normalizes into it.

**D11. `Float32` and `Float64`.** No rule says whether a `Float32` is accepted as a `Float`. **Decision:** `Float32 ≤ Float64` implicitly (exact widening, like integer ranges); mixed arithmetic gives `Float64`; `Float64 → Float32` is the explicit `Float32(x)` (rounding, never fails).

**D12. `Character ≤ String`.** Natively a `Character` is not laid out like a `String`. **Decision:** keep the rule; a copy converts; a view of a collection of `Character` as a collection of `String` is an element-conversion view (`D54`).

**D13. Enumerations: ordering, `parse`, `members`.** **Decision:** not ordered (`TY085`; compare `Integer(a) < Integer(b)`), no arithmetic; `parse` is an exact, case-sensitive match of the member name; `members` is a static `Array(Color)` in declaration order.

**D14. `Fields` identity and limits.** "Same flags, nested the same way" doesn't say whether order matters, but bits follow declaration order and `Integer(fields)` is observable. **Decision:** the ordered tree is the identity; at most 128 leaves; JavaScript uses a `number` up to 32 flags, a `bigint` above.

**D15. Key-set normalization.** "Naming a group is the same as naming all of its members" implies `Omit(T, { foo { a, b } })` = `Omit(T, { foo })` when `foo` has exactly `a` and `b`. **Decision:** key sets are normalized (a complete group is the group); `Omit` of a group removes the field, `Pick` of a group keeps the whole field.

**D16. `is in` with a group.** **Decision:** `x is in s` is true iff every leaf of `x` is in `s`.

**D17. What is comparable.** **Decision:** `==` is defined unless the type contains a function value, a `Task`, an iterator, a reference type or a thread-safe handle (`Mutex`, atomics, `Channel`: identity-like values whose content can change under a reader).

**D18. "Every argument is a type" in the application rule.** Literals are both values and literal types. **Decision:** in this test, literals, object literals (`=`), `{}`, lambdas and value names are values; object *types* (with `:`) are types. `first("a")` is a call.

**D19. Checking generic bodies under a union constraint.** "Checked once against the constraints" with `Value: Integer | Float` doesn't say how `value * 2` or `value / 2` is typed. **Decision:** check each operation once per member and require a uniform result (a fixed type or the member itself, giving `Value`); effects are the union over members.

**D20. Generics on JavaScript.** Erasure breaks for union-constrained numeric parameters (`0` vs `0n`, overflow checks) and for per-type operations (static methods, copies, `==`, hashing, formatting). **Decision:** one emitted instance per bound member for union-constrained parameters whose members behave differently; hidden dictionaries for per-type operations on other parameters; plain erasure otherwise (§3.11.8). No boxing.

**D21. `Partial` and `Required`.** **Decision:** shallow; flags and defaults kept; `Required` of a field typed exactly `None` is `TF024`.

**D22. `Parameters` of a function with variadic, destructured or unnamed parameters.** **Decision:** variadic `...v: T` gives `v: Array(T)`; destructured, anonymous-structure and unnamed parameters are `TF026`.

**D23. Where reference types may appear.** **Decision:** in parameter and return types, relation fields, inferred `use` types, `T in c` inside `Optional` and `Iterator`; nowhere else (`CL022` for collections, `TY073` for an object field that is not a relation side — owned by the relations brief).

**D24. A temporary given to a mutated parameter.** "Calling a mutating function requires a mutable value"; `function-composition.md` makes a temporary to a mutated parameter an error. **Decision:** `EF012` everywhere (the mutation would be lost; one rule for calls and compositions).

**D25. Effects of default values.** **Decision:** defaults run in the callee; their effects are the function's own (declared for block bodies).

**D26. Function-typed parameters inside function types, variadic function types.** The stored bit is inferred from a body, which a function type doesn't have. **Decision:** in a function type, a function-typed parameter is stored (its argument must declare every effect); function types may have a variadic last parameter, matching only a variadic function with the same element type.

**D27. Methods as values.** Not specified. **Decision:** `Human.describe` is a function value whose first positional parameter is the receiver (`items.map(Human.describe)`); a bound method value (`human.describe` without call) is `FN086` (write a lambda): no hidden capture, no `this` surprise.

**D28. `let f = add`.** Only lambdas are forbidden as declaration values. **Decision:** allowed: a plain function value (no named arguments, no defaults, no chain); `function f = add` is the alias form with all of them.

**D29. `catch` around callbacks of non-stored parameters.** "The function can't catch them". **Decision:** a `catch` whose only failing calls are such callbacks is `EF018`; in a mixed left operand, `catch` covers only the callee's own failing calls.

**D30. `with` on a generic function.** **Decision:** type parameters determined by the bound values are fixed; the others remain generic parameters of the result.

**D31. Index arguments of `insert`, `delete`, `take`.** The docs show `users.delete(0)` without `try` and say indexing that isn't proven throws `IndexError`. **Decision:** these methods take an index like `[]` does: `IndexError` unless proven (`insert` accepts `0..=length`); with literal-length facts the examples need no `try`.

**D32. When is a map key proven present?** The fixed range-analysis rules don't cover keys. **Decision:** three fixed rules (literal map with literal keys unchanged since; inside an `includes(k)` guard with an immutable `k`, until the map is mutated; the key of a `getOrInsert` reference). Everything else throws `KeyNotFound`.

**D33. The `key` option of `Set.from`.** Is it part of the type (native wants it inlined; JavaScript needs to know the representation)? **Decision:** part of the value, not the type: native inlines it when the construction is visible and calls it through the header otherwise; JavaScript sets that aren't plain primitive `Set`s are `FaSet` instances implementing the JavaScript `Set` interface, so callers are uniform.

**D34. Result kinds of `filter` and `map`.** Unspecified beyond `Array`. **Decision:** `filter` keeps the collection kind (growing → standard counterpart); `map` returns an `Array(R)`, except on maps, where it maps the values and returns `Map(K, R)`. Set-to-set mapping is `Set.from(set.map(f))`, explicit because it may merge.

**D35. Comparators.** `sort((a, b) => b - a)` must work for floats too, but `Integer | Float` is not a valid type. **Decision:** `sort(Order: Integer | Float)(compare: (a: T, b: T) => Order)`; the sign decides, NaN counts as 0; sorting is stable on every target (JavaScript's is). Natural order (`sort()`) needs `ORDERED` elements and never sorts numbers as strings.

**D36. Callback arguments.** JavaScript passes `(item, index, array)`; Fa lambdas may declare fewer. **Decision:** positional collections pass `(item, index)`, others `(item)`, maps `(key, value)` (the iteration order of tuples); `Map.find` returns a `(K, V)` tuple copy.

**D37. Return values of removals.** **Decision:** `Set.add`/`Set.delete` and `Map.delete` return whether something changed (`Boolean`), `Set.take`/`Map.take` return `T?`, `Pool.delete` returns nothing, `Pool.take`/`Array.take` return the element.

**D38. `item, index` iteration.** The review limited it to `Array`, `GrowingArray` and `String`, but `OrderedColumnar` "has the interface of an `Array`". **Decision:** `OrderedColumnar` too.

**D39. `sum()`.** Used in `arrays.md` (`values.sum()` on a `Set`) but never specified. **Decision:** on collections of a numeric type; result the base type; `0` when empty; overflow follows the arithmetic rules (stop).

**D40. Slice bounds out of range.** `data[2..]` on a short array: clamp (JavaScript) or error? **Decision:** like indexing: bounds must be in `-length..=length`, else `IndexError` unless proven; a start after the end gives an empty array (documented). JavaScript developers get an error instead of a silent clamp, consistent with indexing.

**D41. `Void` in data positions.** "`Void` can be a type argument" vs "can't be stored". **Decision:** allowed as a return type and in `Task(Void)` (and function types); an instantiation that would store `Void` (`Array(Void)` from `map`) is `TY071` with the `for` fix.

**D42. Methods on union receivers.** **Decision:** only universal built-ins (`toString`) without narrowing; user methods require narrowing to a member (no implicit dispatch).

**D43. JavaScript representation of literal members of named unions.** **Decision:** the raw literal (two members can't share a literal), so `MyUnion.Hello` is `"hello"` at runtime, good for JSON and debugging.

**D44. Discriminant narrowing.** The narrowing table doesn't say whether `if shape.kind == "circle"` narrows `shape`. **Decision:** yes, when `shape` is an anonymous union of object members discriminated by `kind` and `shape` is a narrowable place (TypeScript behaviour, no surprise for JavaScript developers).

**D45. Escaping closures and move-only values.** **Decision:** capturing a `Task` or a `destroy` type by copy is `FN072`.

**D46. `String(x)`.** **Decision:** doesn't exist; interpolation and `toString()` are the two documented ways (keep "one way": `CV002` points at `.toString()`).

**D47. `BigInt` to fixed-size integers.** **Decision:** `Integer(b)`, `Int8(b)` throw `ArithmeticError` unless proven.

**D48. Identity of nominal types built by `TypeFromSchema`.** **Decision:** `(the applying declaration, the schema value)`, so equal schemas give the same type and builds stay deterministic.

**D49. Printing and schemas of anonymous types and functions.** **Decision:** `@typeToString` prints the spelling; anonymous objects in declaration order, unions in written order, ranged integers with the differing bounds only, function types with names and effects. `Schema.Function` doesn't describe `requires`, `mutates` or `invalidates` (the documented layout has no place for them); extend `Schema` later if needed.

**D50. Shape of the built-in errors.** `IndexError`, `KeyNotFound`, `DivisionByZero`, `ArithmeticError`, `ParseError`, `Cancelled` are used as types and in `when`, with no declared shape. **Decision:** each is a prelude `Errors` type with one member of the same name: `IndexError { index: Integer, length: Integer }`, `KeyNotFound { key: String }` (the key's `toString()`), `DivisionByZero`, `ArithmeticError { message: String }`, `ParseError { input: String, message: String }`, `Cancelled`. `when error is { IndexError => … }` matches the whole type.

**D51. Lambdas without an expected type.** `defining-functions.md`: a block-bodied lambda "returns nothing when there is none". **Decision:** read "when the expected function type has no return type"; a lambda with no expected type at all (a field of an untyped object literal) annotates its parameters and infers its return type and effects from its body, whatever its form.

**D52. List literals without context mixing types.** **Decision:** same join as `if`/`when` (union of widened types, `TY006` for mixed numeric literals, `TY040` if indistinguishable): `[1, "a"]` is `Array(Integer | String)`.

**D53. Unannotated named-union member values.** `let myCar = MyUnion.Car { … }`: is it a `MyUnion.Car` or a `MyUnion`? **Decision:** the widened type is the union, the member is a fact (T1), so `myCar` can later be reassigned another member if `mutable`, and fields are readable while the fact holds.

**D54. Collection views whose element representation differs** (`Array(Int8)` or `Array("north" | "south")` read as `Array(Integer)` / `Array(String)`). The LLVM views only describe offsets and strides. **Decision:** allowed (they satisfy the rules); native specialization adds an element conversion on load (sign extension, literal table lookup); function-value view tables get a per-leaf load function. JavaScript needs nothing (same representation), except the `bigint`/`number` case of `D2`, which is an error.

**D55. `find` results.** `arrays.md` shows `numbers.find(…) -- Integer?` while sets and pools return `(T in c)?`. **Decision:** `find` returns an optional reference everywhere; `let` copies it into `T?`.

**D56. JavaScript representation of enumerations.** **Decision:** numbers (the member values), name tables for `toString` and `parse`.

**D57. JavaScript property order.** **Decision:** canonical shape order (sorted by name) at every construction and copy, so one shape is one hidden class whatever the spelling; printing uses the static spelling's declaration order.

**D58. Value spread versus type spread.** A value spread lets a later field replace a copied one (with the same type); a type spread never overrides. **Decision:** keep both as documented (update idiom vs composition), and make the difference explicit in the error messages (`TY018` vs `TY101`).

**D59. Options of `from`.** Only `capacity` (arrays) and `key` (sets) are documented. **Decision:** `capacity` for `Array`, `GrowingArray`, `Set`, `Map`, `Pool`, `Columnar` (a hint; ignored on JavaScript), `key` for `Set` and `GrowingSet`; anything else `FN023`.

**D60. JavaScript references must survive whole-value writes.** `arrays.md` (JavaScript): "a reference to an object item is the object itself", while `mutates self[index]` keeps references to the written item valid and they must see the new value. Replacing the JavaScript object breaks this. **Decision:** a write of a whole object or collection value into a place that a live reference may designate is an in-place assignment (`assign$T`), field by field; elsewhere, a plain replacement. Union and optional places are replaced (references into the payload are invalidated anyway).

**D61. Duplicate keys in a map literal.** **Decision:** `CL006` when both keys are compile-time literals (a literal has one meaning); a set literal merges duplicates as documented.

**D62. Members of generic named unions.** Members are "always qualified", but `Result(Integer, String).Ok` is heavy. **Decision:** written `Result.Ok`, arguments inferred like a typed literal; the applied form only when nothing gives the arguments; patterns always use `Result.Ok`.

**D63. Equality of pools and columnars.** "Pools and maps compare without order." **Decision:** multiset equality, relation fields excluded.

**Parser follow-ups found while writing** (not language ambiguities): `T??`; `x is A | B` (must be `x is (A | B)`); `async` function types; `function f: T = …`; `[..]` and `x[i].f` in effect paths; leading-dot member `.red` (verify).

---

## Dependency order

Each step lists what it needs; steps on the same line can run in parallel. Milestone M0 of the spec review (names, scopes, base types, objects, arrays, optionals, `throws`/`try`/`catch`, `when` on named unions) is steps 1–6.

0. **Parser prerequisites**: diagnostics instead of panics; the follow-ups above. Needed before tests can be written as `✗` cases.
1. **Interner and `TypeKind`** (§2): `Sym`, `TypeId`, side tables, builtin ids, flags, stable hashes, `ConstId`. No dependency.
2. **Name resolution and scopes**, type namespace, hoisting; **type-mode resolver** for scalars, literals, objects (shapes, spellings), tuples, optionals, function types without effects. Needs 1.
3. **Assignability** (`Read`, `Copy`), object literals, field access, `TypeDeclInfo` with methods/defaults; declarations and inferred types; joins; **anonymous unions** (normalization, distinguishability). Needs 2. In parallel: **numbers in types** (ranges, storage, literal adaptation; with the sibling brief's numeric rules).
4. **Functions I**: signatures, parameters, calls (positional, named, defaults, variadic, destructuring), return checking, single-expression inference of the return type, recursion rule, lambdas with contextual typing, methods and associated functions, static dispatch. Needs 3.
5. **Named unions, `Errors`, enumerations, `Fields`**; `when`/`is` on them (with the control-flow brief); **`throws`/`try`/`catch`** and the `throws` part of effect inference. Needs 3–4.
6. **Generics**: parameters, application vs call, inference, union-constraint binding and per-member checking, object constraints, method constraints, `Type(parameter)` as an implicit generic, generic methods. Needs 4. Then **`Array`** as a prelude generic with literal typing, `from`, index proofs (needs the range analysis of the sibling brief), sequence methods. → end of M0.
7. **Effects II**: paths (§2.4), `mutates`/`invalidates` inference and checking, write-sets, call-site checks (`EF011`, `EF012`, `EF004`, `EF005`), `requires`, purity, stored bits, closures and captures. Needs 6. Then the **reference checker** (sibling brief / `advanced/reference-checking.md`), which consumes these summaries.
8. **Other collections** as prelude declarations with their effects: `Map`, `Set`, `Pool`, `Columnar`, `OrderedColumnar`, growing variants; key proofs (`D32`); structural variance rules; fusion decision on the typed tree. Needs 6–7.
9. **Type operations and recursion**: spread, difference, recursive canonicalization (§3.6). Needs 3 (spread) and 5 (difference on named unions).
10. **Compile-time interpreter** (metaprogramming brief) → **type functions** (user and built-in), key sets and `@for`, **function expressions** (alias, `then` with chains and slicing, `with`, compile-time factories), **types at runtime** (`@typeToString`, `@typeToSchema`, `TypeFromSchema`). Needs 6, 9.
11. **JavaScript backend**, in the same order: objects (canonical property order, `copy$`, `assign$`, `eq$`, `format$`), unions and `is`, functions, closures and copies of escaping captures, errors, generics (erasure, per-member instances, dictionaries), the collection runtime module (§5.7), fused loops. Can start after step 6 on the M0 subset.
12. **Native**: layouts, monomorphization with view keys, `copy`/`drop`/`eq`/`hash` per shape, unions and niches, closures, then the collections in Fa on `intrinsics` (Array → Map/Set → Pool/Columnar → growing), then attributes from effects (`compilation-with-llvm.md#milestones`). Needs a stable checked tree (steps 1–9).

The test lists of this brief map onto these steps: §3.1–3.4 and §4.1–4.2 tests gate steps 2–4; §3.5, §3.8, §3.9 gate step 5; §3.11 and §5.1 gate step 6; §4.3–4.7 gate step 7; §5.2–5.6 gate step 8; §3.6, §3.10 gate step 9; §3.12–3.14 and §4.10 gate step 10. The same `✓` programs, compiled by each backend, give the end-to-end tests of steps 11 and 12.
