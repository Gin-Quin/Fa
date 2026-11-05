//! The per-file results of the analysis: side tables indexed by node.
//!
//! Types, resolutions, flags, symbols and diagnostics are filled by the walk
//! (`check/`): dense tables indexed by node, then sparse tables for the
//! minority node classes (arithmetic, sinks with a conversion, functions).
//! The backend-facing part is `types`, `resolutions`, `flags`, `arith`,
//! `coercions` and `symbols` (design §2.6), read through the accessors.
//!
//! # What the backends read
//!
//! - [`Analysis::type_of`]: the type of every expression node. A node flagged
//!   [`NodeFlags::TYPE_EXPR`] is a type written in the source (an annotation,
//!   the callee of a conversion `Int8(x)`): its type is the type it denotes.
//! - [`Analysis::resolution`]: what a name refers to. A local symbol's
//!   declaration is `symbols.symbol(id).decl_node`; a parameter is a
//!   [`SymbolKind::Parameter`](crate::symbols::SymbolKind) whose `decl_node`
//!   is the parameter node; the name of a named argument (`speed` in
//!   `move(1, 2, speed = 3)`) resolves to the parameter it fills.
//! - [`Analysis::flags_of`]: the runtime checks to emit (overflow, sinks,
//!   divisors, conversions…).
//! - [`Analysis::arith`] (an [`ArithInfo`] by value) and
//!   [`Analysis::interval`]: the interval, compute type and root bit of an
//!   integer node. The tables store interned [`IntervalId`]s
//!   (`interval_values`); read them through these accessors.
//! - [`Analysis::function`]: per function declaration and per lambda, its
//!   signature, parameters and body ([`FnInfo`]).
//! - [`Analysis::call`]: per call, what it calls and which argument fills
//!   which parameter, defaults and variadic parts included ([`CallInfo`]):
//!   named arguments are already put in parameter order; the receiver of a
//!   method call is [`Argument::Receiver`].
//! - [`Analysis::type_decl`]: per `type` declaration, its fields in
//!   declaration order (the property order of its JavaScript objects) with
//!   their defaults, and its functions ([`TypeDecl`]).
//! - [`Analysis::object`]: per object literal, the type built and every
//!   field in declaration order: given, default or `none` ([`ObjectInfo`]).
//! - Named unions: a member's tag is its index in the nominal type
//!   (`TypeKind::Member(nominal, index)`), its data `TypeStore::payload`; a
//!   member built with its data is a call with `Callee::Member`.
//! - `coercions`: also the object member of an anonymous union a value is
//!   stored as.
//! - Generics: [`Analysis::instantiation`] gives, per call of a generic
//!   function (`FnInfo::generics` order), per generic application
//!   `sum(Float)`, per typed literal or member construction of a generic type
//!   (`TypeDecl::generics` order), and per generic function value given where
//!   a function type is expected (`names.map(identity)`, on the name node),
//!   the types its parameters are bound to. They may contain the
//!   `TypeKind::Param` of an enclosing generic declaration: a backend
//!   monomorphizing substitutes along the chain of calls. The signature of a
//!   generic function uses `TypeKind::Param`; `TypeStore::substitute` with
//!   `Bindings` gives an instance. A generic built-in method (`Array#map`)
//!   has its own parameters (owned by the `prelude` module).
//! - A function value given for a callback type may declare fewer parameters
//!   than the callback (`FnSig::ignorable`, the `index` of collection
//!   callbacks).
//! - Collections: `TypeKind::Collection(kind, item, key)`; for a map, `item`
//!   is the value type and `key` the key type (`NONE` otherwise). A list
//!   literal has the collection type the context expects (`Set`, `Map`,
//!   `GrowingArray`…); the items of a map literal are tuple nodes typed
//!   `Tuple(K, V)`. The members of sets and maps resolve to `Set#…` and
//!   `Map#…` built-ins (shared by `GrowingSet` / `GrowingMap`, told apart by
//!   the receiver's type); `Set.from` and `Map.from` calls are
//!   `Callee::Builtin` with their arguments in the order `(source, key,
//!   capacity)` for sets and `(source, capacity)` otherwise, the type name
//!   before `.from` flagged `TYPE_EXPR` with the built collection type. A
//!   method type of a collection carries its effects (`FnSig::effects`, rooted
//!   at `PathRoot::SelfParam`, the first argument as `PathStep::IndexParam(0)`);
//!   `builtins::array::effects(kind, "[]=")` gives those of an element write.
//! - `map[key]` reads are flagged `INDEX_CHECK | CAN_FAIL` when the key is
//!   not proven present: the backend throws `KeyNotFound` (types brief D50).
//!   `map[key] = value` never fails and has no flag.
//! - `for (key, value) in map`: the loop's `TuplePattern` node and its
//!   names are typed `Tuple(K, V)`, `K` and `V`.
//! - Recursive types (types brief §3.6): inside the type of a declaration
//!   that names itself, the reference is a `TypeKind::Rec`; it always sits
//!   behind an indirection (a union member, an optional, a collection
//!   element: `TY060` otherwise), and it is the **boxed** edge of the native
//!   layout (a heap block owned by its parent; `none` is the null pointer).
//!   `TypeStore::rec_target` (or `unfold`) gives the type it stands for;
//!   `rec_info` its declaration and type arguments. Types on a cycle have the
//!   `RECURSIVE` flag: their copy and drop must be iterative. Two recursive
//!   declarations with the same unfolding are assignable both ways
//!   (coinductive), but keep distinct ids.
//! - Object literals with spreads (`{ ...defaults, port = 8080 }`): the
//!   fields copied from a spread are `FieldValue::Spread(value)` in
//!   `ObjectInfo::fields`; evaluate the spread values and the written values
//!   in source order. Type operations (`A - B`, spreads in types, `Pick`,
//!   `Omit`, `Partial`, `Required`, `KeyType`) are resolved: the types are
//!   plain object, union and masked named-union types. A spread in a `type`
//!   declaration copies the fields into its `TypeDecl::fields`, with the
//!   default value nodes of the spread type's declaration.
//! - Structural typing (advanced brief §10): a value copied into a place of
//!   another object type (a binding, a field, a collection element, a
//!   `push`/`add` argument, a return) has `coercions[value]` = the target
//!   type: copy only the target's fields (element by element for a
//!   collection, `copy_Destination`). An argument of a user function is a
//!   view: no coercion. A method call resolves to the function of the type
//!   **written** for the receiver (its annotation or the literal it was
//!   built with), so `human.describe()` and `animal.describe()` may call
//!   different functions on one shape: follow `resolution(member)`.
//! - `Type(parameter)` in a signature makes the parameter an implicit generic
//!   parameter (a `TypeKind::Param` named `Type(name)`, constrained by the
//!   written type) appended to `FnInfo::generics`; calls record its
//!   instantiation like any generic. `Type(value)` elsewhere is the value's
//!   type.
//! - A constraint may require a method (`Item: { describe: (self) => String
//!   }`): the field has `FieldFlags::METHOD` and the method's type without
//!   `self`. In the generic body, `item.describe()` resolves to that field
//!   (`Resolution::Field`); a backend monomorphizing calls the function of
//!   the actual type found by name (types brief §3.11.6).
//! - Compile-time value parameters (`rows: Integer`, `ParamInfo::value`):
//!   their binding in `instantiations` is the literal type of the value
//!   (`Matrix(Float, 2 + 2, 4)` binds `rows` to the type `4`); in the body,
//!   the parameter's name is a `SymbolKind::Static` of its declared type,
//!   whose value comes from the instantiation.
//! - The number API is `Resolution::Builtin` on the member node, recognized
//!   by its path (`builtins/numbers.rs`): methods `Integer#…` (every integer
//!   type: `toString`, `abs`, `clamp`, `bitAnd`/`bitOr`/`bitXor`/`bitNot`,
//!   `shiftLeft`/`shiftRight`, `wrapping…`/`saturating…`/`checked…`
//!   `Add`/`Sub`/`Mul`/`Pow`), `Float#…` (`Float` and `Float32`: `round`
//!   halves away from zero, `floor`, `ceil`, `truncate`, `sqrt`, `abs`,
//!   `clamp`, `isNaN`, `isFinite`, `toString`), `BigInt#…`, `Percentage#…`;
//!   their result has the receiver's type (its storage type for an integer).
//!   `x.abs()` on an integer is an arithmetic node (`arith`, overflow bits).
//!   Constants are values: `Int8.minValue`… have a literal type in their
//!   storage (the value is the type's), `Float.nan`, `Float.infinity`,
//!   `Float.maxValue`, `Float.epsilon` (and `Float32.…`) are `Float`s.
//!   `Integer.round(x)` / `floor` / `ceil` / `truncate` convert a float and
//!   throw `ArithmeticError` (NaN, infinite, out of range) unless the analyzer
//!   proved them (a literal that fits): `CONVERSION_CHECK` + `CAN_FAIL` when
//!   they can fail. A shift has `SHIFT_CHECK` + `CAN_FAIL` when its amount is
//!   not proven in `[0, width)`. `Math.max(a, b)` / `Math.min(a, b)` are calls
//!   with `Callee::Builtin`, two `Argument::Given`, the result has the shared
//!   kind (the base integer type, `interval` when narrower). `Float.parse`
//!   throws `ParseError`.
//! - `Array.from(source, capacity = n)` / `Array(T).from(…)`: a call with
//!   `Callee::Builtin` (`Array.from`) and the arguments `[source, capacity]`
//!   (`Argument::Default` without capacity); it copies the source's items
//!   into a new array of the call's type; `capacity` is a hint.
//! - Capacity (minimal representations, `design/decisions.md`): every
//!   collection's `from` is a call with the arguments `(source, capacity)`
//!   (`(source, key, capacity)` for sets), `Argument::Default` without
//!   `capacity`; `String.from(text, capacity = n)` is `Callee::Builtin`
//!   with `[text, capacity]`. `capacity` is a property (`Array#capacity`,
//!   `Set#capacity`, `Map#capacity`, `Pool#capacity`, `Columnar#capacity`,
//!   `OrderedColumnar#capacity`, `String#capacity` in bytes; the growing
//!   collections share their counterpart's path) and `reserve(additional)`
//!   a method (`…#reserve`, `String#reserve`). `reserve` declares `mutates
//!   self` on an `Array`, a `Set`, an `OrderedColumnar` and a `String` (it
//!   may move the elements or the bytes), `mutates self.capacity`
//!   (`PathStep::Capacity`, `flow::Step::Capacity`) on the collections whose
//!   elements never move: it invalidates nothing there. JavaScript: `reserve`
//!   does nothing, `capacity` is the `length` (or `size`).
//! - Text: `characters()` keeps the type `Array(Character)`; its native
//!   representation is a view of the string's bytes when the array is only
//!   read (indexed, measured, iterated, a `let` only read), like `bytes()`
//!   and `codePoints()`: the backend decides from the syntax, as it does
//!   for them. An `Array(Character)` the program keeps and changes is built.
//! - [`Analysis::is_non_escaping`] (`non_escaping`): the `let` / `mutable`
//!   locals of blocks and functions whose value never outlives the frame of
//!   their function (never returned, thrown, stored, moved into a detached
//!   task or into a local that escapes; `check/locals.rs`): natively, a
//!   short string among them may live in a stack buffer.
//! - [`Analysis::module_constant`] (`module_constants`): the module-level
//!   `let` values whose value is a constant expression (literals, constant
//!   arithmetic, templates, lists, tuples and objects of constants, other
//!   constants and statics), with that value: fold their reads or store
//!   them once in read-only data. Their declarations and reads stay valid
//!   code (unlike [`Analysis::constant`]): the JavaScript backend keeps them.
//! - `WorkerPool(T) { count, state, maxQueueSize, timeout }` (`std.concurrency`,
//!   `TypeKind::Sync(SyncKind::WorkerPool, T)`, `T` a worker type): an
//!   [`ObjectInfo`] of those four fields, `FieldValue::None` when left out;
//!   a missing `state` (allowed when `T {}` is a value) means `T {}`, each
//!   worker starting from its own copy (and again after a `timeout`).
//!   `pool.submit(task)` (`WorkerPool#submit`): `task` is an `async`
//!   function receiving a free worker (`Sync(Worker, T)`), the call a
//!   `Task(R) throws E | WorkerTimeout`, `E` the errors of `task`; without
//!   `WorkerTimeout` when the pool is a local declared without `timeout`
//!   (`concurrency.rs`, `submit_without_timeout`); `pool.shutdown()`
//!   (`WorkerPool#shutdown`).
//!   On `native`, `bun`, `node` and `web` (`CONC010` elsewhere; `submit`
//!   is blocking only natively): see "Worker pools" below.
//! - Parameters are never copied (minimal representations): every parameter
//!   is a reference to the caller's place or temporary; the effects say
//!   which ones the callee changes (`FnSig::effects`), and a callee copies a
//!   parameter only where it stores it (a copy site, `MOVE` when it can
//!   move instead).
//! - Tuples: `TypeKind::Tuple`; a tuple value is `View::Tuple` (in a group);
//!   `t[0]` is an `Index` whose index is an integer literal (in range,
//!   checked), never `INDEX_CHECK`. Equality is element-wise.
//! - Ranges: `TypeId::RANGE` values (`a..b`, `..=`, open bounds); the fields
//!   are `Range#start`, `Range#end` (`Integer?`) and `Range#includesEnd`,
//!   readable and assignable on a `mutable` range. `for i in range` over a
//!   `Range` value (not a literal) iterates from `start` (a missing start
//!   stops the program) by one. `items[range]` / `text[range]` is a slice (an
//!   `Index` whose index has the type `Range`): the array type / `String`,
//!   `INDEX_CHECK` + `CAN_FAIL` (`IndexError`) unless proven; bounds go from
//!   `-length` to `length`, missing ones are the ends, an inverted range is
//!   empty. `x is in range` (a `Range` value) is a membership test.
//! - Destructuring ([`Analysis::pattern`]): per pattern node of a `let` /
//!   `mutable` declaration, a `for` variable or a destructured parameter.
//!   A destructured parameter is a `FnParam` whose node is the
//!   `DestructuredParameter` and whose symbol is anonymous (named by the
//!   pattern's text): bind its pattern from that parameter.
//! - Effects ([`crate::effects`]): `FnSig::effects` of a named function or
//!   method holds its `mutates` / `invalidates` paths, declared or inferred
//!   (`TypeStore::effects`, `effects::path_text`); a lambda's type carries
//!   only the effects written on it. Effects need no code: a path root
//!   `PathRoot::Param(i)` indexes `FnInfo::params` (`self` included,
//!   `PathRoot::SelfParam` for `self`). The LLVM backend may read them for
//!   attributes: a pointer parameter that is the root of no path is
//!   read-only during the call, and two parameters never alias (the alias
//!   rule, `REF013`).
//! - References: `use x = place` and `mutable use x = place` are
//!   `View::Use` nodes declaring a symbol of kind `Use` / `MutableUse` whose
//!   `decl_node` is that node; its value (flag `PLACE`, through a `try`) is
//!   the place the symbol designates. Reading or writing `x` reads or
//!   writes that place, evaluated once at the declaration (advanced brief
//!   §6.6: a JavaScript object is its own reference; a primitive place is a
//!   container and a key). `reuse x = place` (`View::Reuse`, its node
//!   resolves to the `mutable use` symbol) designates a new place from
//!   there on; `reuse x = none` an absent one.
//! - A `for` variable over a collection (`SymbolKind::LoopVariable { copy:
//!   false }`) designates the current item: an assignment to it (`slot =
//!   sum`, `score += 1`) or below it (`body.vx -= …`) writes the item in the
//!   collection (JavaScript: an index loop over a collection of primitives).
//!   The collection's length can't change during the loop (`REF012`).
//! - Requirements: `FnSig::requires` lists a function's requirements
//!   (declared, and inferred for a single-expression function), sorted by
//!   the stable hash of their key type (decision D11): one hidden
//!   parameter each, after the regular ones, in that order. Inside the
//!   body, a requirement is a symbol of kind `Requirement { index }`.
//!   [`Analysis::requirement_args`] gives, per call, what fills each one
//!   ([`RequirementArg`]): the value of a `provide` statement
//!   (`View::Provide`: an alias of a place, or a temporary evaluated once
//!   at the statement), or the caller's own requirement of that key.
//! - Closures: `FnInfo::captures` lists the locals of enclosing functions a
//!   lambda or a local `function` uses; [`Analysis::escapes`] the ones that
//!   escape (given to a method that stores them, assigned to a place,
//!   returned; not yet: a field of an object literal): those copy their
//!   captures when they are created; the others use the variables
//!   themselves (they run before their creator returns).
//! - [`crate::lower`] reads these tables for the backends: the decision
//!   list of a `when` and the names its arms bind, how an index is checked
//!   and normalized, when a stored value is copied (D15, `MOVE`), and the
//!   order in which the arguments of a call are evaluated.
//! - Iterators (types brief §5.0.6): `values()` of a collection (`Array#values`,
//!   `Set#values`, `Map#values`, `Map#keys`, `Pool#values`, `Columnar#values`,
//!   `OrderedColumnar#values`) gives a `TypeKind::Iterator(item, _)`; its
//!   methods are `Iterator#filter`, `#map`, `#find`, `#reduce`, `#every`,
//!   `#some`, `#firstValues` (`count`, default 1) and `#toArray`, lazy and
//!   element by element (callbacks receive the item only). `for x in
//!   c.values()` (no `map` in the chain) designates the elements of `c`: a
//!   write through `x` writes the element.
//! - Pools and columnars (types brief §5.5, §5.6): `Pool#…` (also for a
//!   `GrowingPool`), `Columnar#…` (also `GrowingColumnar`) and
//!   `OrderedColumnar#…`, told apart by the receiver's type. `add(value)`
//!   stores a copy (an object literal is built in place) and returns the
//!   element: a reference when bound by `use` (the `use` designates the new
//!   element); `delete(element)` / `take(element)` of a pool or a columnar
//!   take a reference into the receiver (`REF020`, alias `CL050`, otherwise); an
//!   `OrderedColumnar` is positional (`delete(index)`, `pop`, `c[i]`).
//!   Iteration order is the slot order. `==` compares as multisets (D63).
//! - Relations (advanced brief §8, `relations.md`): [`Analysis::relations`]
//!   lists the relations declared in the file ([`Relation`]: kind, the types
//!   and fields of both sides, the holder and target collections);
//!   [`Analysis::relation_fields`] says which fields of a type declaration
//!   are relation sides. A direct side `(T in c)?` and a reverse one-to-one
//!   or symmetric side are typed `T?`; a many-to-many direct side and the
//!   `Pool` reverse sides are `Collection(RelationPool(0), T)` (direct,
//!   writable: `RelationPool#add`/`#delete`/`#clear` link and unlink, `add`
//!   and `delete` return whether something changed) and
//!   `Collection(RelationPool(1), T)` (reverse, read-only). An object
//!   literal leaves relation fields out (`FieldValue::None`): a new element
//!   starts unlinked, and copies (`let`, `take`, another collection) have no
//!   links and empty relation fields (D18); equality and printing skip
//!   relation fields (D21). [`Analysis::relation_writes`] marks every write
//!   of a direct side (`x.f = t`, `x.f.add(t)`…): compile it into the
//!   two-sided update of §8.5 for the relation's kind. A literal given to
//!   `add` may set direct sides (and a many-to-many side with a list of
//!   references, D19): link them once the element is stored.
//!   [`Analysis::relation_unlinks`] marks every removal from a collection
//!   that relations hold or target (`delete`, `take`, `deleteWhere`,
//!   `clear`): unlink each removed element first (§8.5 `unlink`: the direct
//!   sides that pointed at it become `none` or lose it, it leaves the reverse
//!   sides of its targets, the owner fields into the collection become
//!   `none`); these writes are the implied effects of the removal (§8.6).
//!
//! # Compile-time values (advanced brief §12)
//!
//! The analysis runs compile-time code on the interpreter (`fa-interpreter`,
//! `comptime/`, `check/comptime.rs`) for the target it is parameterized by
//! (`Session::set_target`, D22). What the backends read:
//!
//! - [`Analysis::constant`] (`constants`): a node whose value is known at
//!   compile time, with that value (a [`fa_interpreter::Value`]): an `@`
//!   call (`@add(2, 4)`), the read of a `static` (and `settings.port` of
//!   one), `target`, `configuration`, `@readFile(…)`, `@parseJson(…)`,
//!   `@typeToString(…)`. **Check it before generating any expression
//!   node** and emit the value instead (`static foo = 12;
//!   console.log(foo)` is `console.log(12)`): the node's own code is never
//!   run (an `@` call's callee may not even be compiled). The node's type
//!   (`type_of`) says how to represent the value (an `Int64` is a bigint on
//!   JavaScript, an anonymous object keeps the value's field order). The
//!   values are plain data: `None`, `Boolean`, `Integer` (`I256`), `Float`,
//!   `Percentage`, `String`, `Array`, `Tuple`, `Object` (fields in
//!   declaration order), `Set`, `Map`, `Member` (a named union, enumeration
//!   or error member, by qualified name), `Range`; never a function. A
//!   value a backend can't write yet is its error, not a silent fallback.
//! - A `static` declaration (`View::Declaration { kind: Static }`) emits
//!   nothing: its value is in `static_values` (by symbol) and on its node.
//! - [`Analysis::comptime_branch`] (`comptime_branches`): per `@if`, the
//!   branch kept (`true`: the first, `false`: the `@else`). Only that branch
//!   is analyzed; the other has no types and must not be generated.
//! - `unrolled`: per `@for` whose items are known, the items: emit the body
//!   once per item with the loop variable (and the index) bound to it. An
//!   `@for` without an entry (a range bounded by a compile-time parameter of
//!   a generic function) is an ordinary loop.
//! - `@test` blocks ran during the analysis (`TEST003` on failure): they
//!   are not part of any output.
//! - `build_inputs`: the files and folders compile-time code read, relative
//!   to the root (a folder ends with `/`): what a build cache must hash.
//! - `reads_target`: the analysis read `target` or `configuration`, so it
//!   differs per target (`Session::depends_on_target`; `fa check` analyzes
//!   such programs for every configured target).
//!
//! # Async (`design/async.md` §2, `check/tasks.rs`)
//!
//! The analysis types `async`, `await`, `start`, `start do` and `Task`;
//! the backends generate them (steps N1 and W1: the LLVM backend,
//! `fa-codegen-llvm/src/lower/tasks.rs`; step J1: the JavaScript backend,
//! `fa-codegen-js/src/generate/tasks.rs`). A backend that doesn't yet
//! reports `CONC011` ("async code generation is coming") on `await` and
//! `start`; an `async` function that is only called blocking is a plain
//! function.
//!
//! - **Function types**: `FnSig::is_async`. An `async` function value
//!   carries both instances (`{sync, async, env}` natively, two properties
//!   on JavaScript); in synchronous mode only `{sync, env}`.
//! - [`Analysis::async_summaries`]: per function declaration and lambda
//!   (both keys for a declaration), per `start do` node, and for the top
//!   level (the `Module` node) and each test (the `test` node): its
//!   [`AsyncSummary`]: `suspends` (it may suspend when awaited, so it needs
//!   an async instance, A4), `starts` (a `start` is reachable from its
//!   body), `blocking` (the targets where its synchronous instance can
//!   exist). The summaries of a file's functions are also in its interface
//!   (`Interface::async_summaries`, by declaration node), and part of its
//!   hash. A function whose `suspends` is false has **one** instance:
//!   `await f()` of it is a plain call.
//! - [`Analysis::awaits`]: per `await` node, an [`AwaitInfo`]: what it
//!   awaits (a call, whose callee instance the backend chooses; a `Task`; a
//!   function value; a primitive), whether it may suspend, whether
//!   `Cancelled` was added to its errors (A11), and whether its operand is a
//!   task-returning expression awaited right away (`await start f()`,
//!   `await Task.all(…)`: call the async instance directly, A15).
//! - [`Analysis::starts`]: per `start` node, a [`StartInfo`]: detached or
//!   kept, the `do` block of a `start do`, the locals it captures, the
//!   values a detached task copies (moved at their last use, A12), and the
//!   scope that owns a kept task (its drop point: leaving it cancels the
//!   task and waits for it). The requirements of a started call are in
//!   `requirement_args` like any call's: a kept task borrows them.
//! - `Task(T) throws E` is `TypeKind::Task(T, E)`: move-only (`MOVE_ONLY`).
//!   `task.cancel()` is the built-in `Task#cancel`; `Task.all`, `Task.any`
//!   and `Task.settled` (calls with `Callee::Builtin` and one
//!   `Argument::Given`: the array of tasks, moved in) and `Task.yield()`
//!   are built-ins of the `Task` namespace. `Task.all` gives
//!   `Task(Array(T)) throws E`, `any` `Task(T) throws E`, `settled`
//!   `Task(Array(T | E))`. A `Task` value is the frame or the finished
//!   block of the task (`design/async.md` §3.2).
//! - **Instance selection** (mechanical, in the backend): a call node uses
//!   the async instance of its callee when all of these hold: it is
//!   awaited or started (`awaits` / `starts` on its parent), the enclosing
//!   instance is an async instance, the program is asynchronous
//!   (`Program::asynchronous`), and the callee's summary `suspends`.
//!   Otherwise it uses the synchronous instance. Instances are generated
//!   on demand from the entry, each one cached separately. A non-async
//!   function containing `start` checks the thread's blocking depth (`start`
//!   is eager in a blocking call); in synchronous mode the depth starts at 1.
//! - **Interleaving points** of an async instance: every suspending
//!   `await`, and every loop back-edge (the lead's amendment of A2: the
//!   backend decrements the thread's coop budget there, and re-queues the
//!   task when it is spent and the turn lasted more than 2 ms, like `await
//!   Task.yield()`). The analysis forgets what it knew about module-level
//!   `mutable` values at both. Synchronous instances never yield. The
//!   top level of a program and the tests (async contexts) yield at their
//!   back-edges too; the analysis forgets there only the module-level
//!   values other code may change (`check/tasks.rs`, `loop_back_edge`).
//! - `Program::asynchronous`: the resolved mode (`build.asynchronous`, or
//!   the detection of A6: the entry's transitive summary `starts` or
//!   `suspends`); `Program::asynchronous_reason` says why (`fa build -v`).
//!
//! ## The I/O of the standard library (`design/async.md` §5, step S1)
//!
//! The built-ins below are calls with `Callee::Builtin` (registry paths in
//! `builtins/std_library.rs`). Those that wait are `async` primitives: a
//! call is blocking (the synchronous version, where it exists on the
//! target: `CONC010` elsewhere), or awaited (`AwaitKind::Primitive(name)`
//! with the name given here), or started (`start File.read(path)`: a task
//! of the primitive). Each backend implements both versions where the
//! table says so.
//!
//! | Path | Await name | Blocking on | Signature |
//! | --- | --- | --- | --- |
//! | `std.time.sleep` | `sleep` | native, bun, node | `(milliseconds: Integer)` |
//! | `std.time.monotonic` | (no wait) | every target | `(): Integer`, nanoseconds of a monotonic clock |
//! | `File.read` | `File.read` | all but web | `(path: String): String throws FileError` |
//! | `File.readBytes` | `File.readBytes` | all but web | `(path: String): Array(Byte) throws FileError` |
//! | `File.write` | `File.write` | all but web | `(path: String, text: String) throws FileError` (creates or replaces) |
//! | `TcpListener#accept` | `TcpListener.accept` | native | `(): TcpStream throws NetError` |
//! | `TcpStream.connect` | `TcpStream.connect` | native | `(host: String, port: Integer): TcpStream throws NetError` |
//! | `TcpStream#read` | `TcpStream.read` | native | `(buffer: Array(Byte)): Integer throws NetError` |
//! | `TcpStream#write` | `TcpStream.write` | native | `(bytes: Array(Byte), count = bytes.length) throws NetError` |
//! | `TcpListener#close`, `TcpStream#close` | (no wait) | every target with sockets | `()`, consumes the socket |
//! | `TcpListener#port` | (a property) | | `Integer` |
//!
//! - **Sockets** are `TypeKind::Sync(SyncKind::TcpListener | TcpStream,
//!   NONE)`: move-only resources (`MOVE_ONLY`; `let other = stream` moves
//!   it, `close()` consumes it), closed when dropped. They don't exist on
//!   `web` and `wasm` (`AN004`, `std_library::missing_on`).
//! - **`TcpListener { host = "…", port }`** (an `objects` entry like the
//!   literals of `std.concurrency`; `host` may be omitted: `"127.0.0.1"`)
//!   listens when it is evaluated; a failure (a port taken, an unknown
//!   host) is thrown by its first `accept()`. `port = 0` takes a free
//!   port, which `listener.port` gives. Hosts are IPv4 addresses or
//!   `localhost`.
//! - **`stream.read(buffer)`** mutates its argument (an effect on the
//!   buffer's place, `EFF007` when it isn't writable; the backend passes
//!   the array itself): it fills the first elements and gives their count,
//!   0 once the other side closed the connection. **`write`** returns once
//!   every byte is written.
//! - **Errors**: `NetError { message }` (`"connect: Connection refused"`:
//!   the operation, then the system's text) and `FileError { path, message }`.
//!   The id of a built-in error is its position in `builtins::errors::ERRORS`
//!   plus one (`NetError` is the last one).
//! - Awaiting a call of these in a synchronous instance (a blocking call,
//!   or asynchronous mode off) is the blocking version.
//!
//! ## Worker pools (`design/decisions.md`)
//!
//! The pools of the minimal-representations section above, for the native
//! backend: a missing `state` is built from the `objects` entry of the
//! literal's type node (the `target` of its `TypedObject`), the object
//! `T {}` with the defaults of `T`'s fields (recorded when they are all
//! known). The task given to `submit` escapes (its captures are copies),
//! and `submit` starts a task (`starts`). It follows the rules of worker
//! code (`CONC008`: no module-level `mutable` value that isn't
//! thread-safe, through every function it calls: the program's check,
//! from `submitted_works`). Workers and pools also exist on Bun, Node and the web
//! (step J2: `worker_threads`, Web Workers), where a call of a worker is
//! awaited or started: blocking, it is `CONC010` off `native`.
//!
//! # Reactivity (advanced brief §11, `check/reactivity.rs`)
//!
//! Values stay plain: a reactive place has a side *source* (a version and
//! observers), and only the places listed here are instrumented. A file
//! without `reactive`, `derived` nor `watch` has empty tables.
//!
//! - Declarations: `View::Declaration { kind: Reactive }` is a `mutable`
//!   value with its own source (`ReactiveRoot::Value`);
//!   `View::Declaration { kind: Derived }` is a node computing its value
//!   lazily: every read of its symbol (a `SymbolKind::Derived`) reads the
//!   node, brought up to date (it tracks itself: derived reads are not in
//!   `reactive_reads`). Its value and the body of a `watch` are closures
//!   run later: they copy the non-reactive locals they use when declared
//!   (escaping closures, §5.11), except the values whose reactive fields
//!   they read.
//! - A `reactive` field: `FieldFlags::REACTIVE` on the field; its source
//!   belongs to its object (created on demand, never copied with it).
//! - [`Analysis::reactive_reads`]: per read node (the outermost place
//!   expression; for a built-in method reading its receiver as a whole, the
//!   callee), the [`ReactivePlace`] to track before the value is read. Only
//!   the reads that may run while a derived value or a `watch` block is
//!   computed are listed (function bodies always are).
//! - [`Analysis::reactive_writes`]: per assignment, compound assignment or
//!   call of a built-in method, the places to notify after the write. A
//!   flat value equal to the previous one changes nothing; a new map key
//!   also changes the size of its map.
//! - [`Analysis::reactive_args`] (D24): per call of a function of this file
//!   given a reactive place (a non-flat parameter receives the place), the
//!   place of each parameter's argument. The callee then needs an instance
//!   that receives the sources of its parameters
//!   ([`ReactiveRoot::Param`] places are instrumented in that instance
//!   only, and give no tracking in the plain one). The callee notifies the
//!   writes it makes through them: the call itself is not in
//!   `reactive_writes`.
//! - [`Analysis::flush_after`]: the statements of the outermost running code
//!   (top level, test bodies, the entry function) after which the pending
//!   `watch` blocks run (`faFlush()`); a statement elsewhere never flushes.
//! - [`Analysis::watches`]: per `watch` node, its body, its cleanup (the
//!   lambda that is its last statement: returned by each run, it runs
//!   before the next one and when the block is destroyed) and whether it is
//!   a `Subscription` (`Subscription#stop` destroys it). A `watch` lives
//!   until the end of its block (the program, at the top level of the
//!   module), and runs once when declared.

use bitflags::bitflags;
use rustc_hash::FxHashMap;

use crate::diagnostics::Diagnostic;
use crate::ids::{
	BuiltinId, FileId, IntervalId, ListId, NodeId, ParamId, ScopeId, SigId, Sym, SymbolId, TypeId,
};
use crate::intern::Interner;
use crate::symbols::{Resolution, Symbol, SymbolTable};
use crate::types::{ComputeType, FieldFlags, Interval};

bitflags! {
	/// Per-node facts for the backends (design §2.6).
	#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
	pub struct NodeFlags: u32 {
		/// The node may exceed the native compute type (values brief §6.7).
		const OVERFLOW_NATIVE = 1 << 0;
		/// A number-represented node may exceed ±(2^53 − 1).
		const OVERFLOW_JS = 1 << 1;
		/// The root value must be range-checked against its sink type.
		const SINK_CHECK = 1 << 2;
		/// The divisor is not proven non-zero: `DivisionByZero` path.
		const DIVISOR_CHECK = 1 << 3;
		/// The index or slice is not proven: `IndexError` path.
		const INDEX_CHECK = 1 << 4;
		/// The conversion is not proven to fit: `ArithmeticError` path.
		const CONVERSION_CHECK = 1 << 5;
		/// The exponent is not proven non-negative.
		const EXPONENT_CHECK = 1 << 6;
		/// The shift amount is not proven in `[0, width)`.
		const SHIFT_CHECK = 1 << 7;
		/// `INT_MIN // -1` and `INT_MIN %% -1` guards are needed (values brief D-49).
		const MIN_DIV_GUARD = 1 << 8;
		/// A payload access without re-testing the tag. A narrowed name in a
		/// place position (the receiver of a method, the root of a field or
		/// item place, a compound assignment, a `mutates` argument, a `use`)
		/// designates the payload inside the optional's or union's own slot,
		/// never a copy: writes through it change the original. A name
		/// assigned as a whole is not `NARROWED` (it has its declared type).
		const NARROWED = 1 << 9;
		/// Unreachable code.
		const UNREACHABLE = 1 << 10;
		/// A type expression: its type is the type it denotes.
		const TYPE_EXPR = 1 << 11;
		/// A constant folded by the analyzer.
		const CONST_FOLDED = 1 << 12;
		/// A place (assignment target, `use`, effect argument).
		const PLACE = 1 << 13;
		/// The read of a local (a name) is its last use, in a copy position
		/// (the value of a `let`/`mutable` or an assignment, a returned
		/// value, an item given to `push`/`add`): the value can be moved
		/// instead of copied (native: no copy and no drop of the source;
		/// JavaScript: no copy, D15). Only for values that a copy would
		/// duplicate (objects, collections, strings, unions holding them).
		const MOVE = 1 << 14;
		/// The expression can fail with an error.
		const CAN_FAIL = 1 << 15;
		/// The explicit generic arguments of a generic function, `sum(Float)`
		/// in `sum<Float>([1, 2])` (D18): an application, not a call. The
		/// applications of types are `TYPE_EXPR`.
		const GENERIC_APPLICATION = 1 << 16;
	}
}

impl NodeFlags {
	/// The flags a `-- no-check` annotation forbids: every runtime check.
	pub const CHECKS: NodeFlags = NodeFlags::OVERFLOW_NATIVE
		.union(NodeFlags::OVERFLOW_JS)
		.union(NodeFlags::SINK_CHECK)
		.union(NodeFlags::DIVISOR_CHECK)
		.union(NodeFlags::INDEX_CHECK)
		.union(NodeFlags::CONVERSION_CHECK)
		.union(NodeFlags::EXPONENT_CHECK)
		.union(NodeFlags::SHIFT_CHECK);
}

/// The analysis of one file.
pub struct Analysis {
	/// The analyzed file.
	pub file: FileId,
	/// The type of every node (`UNSET` for nodes without one).
	pub types: Vec<TypeId>,
	/// What every name or member node refers to (`Unresolved` for other nodes).
	pub resolutions: Vec<Resolution>,
	/// The flags of every node.
	pub flags: Vec<NodeFlags>,
	/// The symbols and the scope arena of the file.
	pub symbols: SymbolTable,
	/// Parse diagnostics first, then the analysis' own, sorted by span.
	pub diagnostics: Vec<Diagnostic>,

	// ── sparse tables (design §2.6): only the nodes of a minority class ──
	/// Every integer arithmetic node (`+ - * // %% **`, unary `-`, compound
	/// assignment) and every range-checked sink: its interval (an id into
	/// `interval_values`) and compute type. Read it with [`Analysis::arith`].
	pub arith: FxHashMap<NodeId, ArithEntry>,
	/// The implicit conversion of a value at its sink, by value node: the
	/// target type of a union injection, `Integer` ↔ `Int64`, `Float32` →
	/// `Float`, `Character` → `String`, and of a `SINK_CHECK` range check.
	pub coercions: FxHashMap<NodeId, TypeId>,
	/// Integer nodes whose flow fact is narrower than their type (hover,
	/// proofs): an id into `interval_values`. Read it with [`Analysis::interval`].
	pub intervals: FxHashMap<NodeId, IntervalId>,
	/// The intervals of `arith` and `intervals`, interned (most nodes share
	/// a few intervals: 4 bytes per node instead of 64).
	pub interval_values: Interner<Interval>,
	/// The type arguments of a generic call, typed literal or generic function value (M2).
	pub instantiations: FxHashMap<NodeId, ListId>,
	/// Every function declaration and lambda node (both keys for a
	/// declaration whose value is a lambda).
	pub functions: FxHashMap<NodeId, FnInfo>,
	/// Every call node: its callee and its arguments in parameter order.
	pub calls: FxHashMap<NodeId, CallInfo>,
	/// Per call of a function with requirements: what provides each one, in
	/// the order of the callee's `FnSig::requires` (sorted by the stable
	/// hash of their keys, decision D11): the hidden arguments, after the
	/// regular ones.
	pub requirement_args: FxHashMap<NodeId, Vec<RequirementArg>>,
	/// The closures that escape (lambda nodes; for a local `function`, its
	/// lambda): stored in a collection, returned. They copy the values they
	/// capture (`FnInfo::captures`) when created; the others see the
	/// captured variables themselves (advanced brief §5.11, D3).
	pub escapes: rustc_hash::FxHashSet<NodeId>,
	/// The `let` and `mutable` locals of blocks and functions whose value
	/// never outlives the function's frame ([`Analysis::is_non_escaping`]).
	pub non_escaping: rustc_hash::FxHashSet<SymbolId>,
	/// Every `type` declaration node: its type, fields in declaration order,
	/// functions.
	pub type_decls: FxHashMap<NodeId, TypeDecl>,
	/// Every object literal node (typed or not, and named-union members built
	/// with braces): its type and its fields in declaration order.
	pub objects: FxHashMap<NodeId, ObjectInfo>,
	/// The default values of the fields of types declared in other files,
	/// which their files evaluated at compile time ([`FieldValue::Constant`]).
	pub field_constants: Vec<fa_interpreter::Value>,
	/// The default values of the fields of the object types `Json.parse`
	/// reads that are declared in other files: their constants
	/// ([`Analysis::field_constants`]), by type and field name.
	pub json_defaults: FxHashMap<(TypeId, crate::ids::Sym), u32>,
	/// The reads that move a move-only value out of its place (a local, a
	/// parameter, a field of one: `holder.task` returned or passed): the
	/// place gives the value up (a zero, which its drop skips).
	pub moved_reads: rustc_hash::FxHashSet<NodeId>,
	/// Every destructuring pattern node (`ObjectPattern`, `ArrayPattern`,
	/// `TuplePattern`) of a declaration, a `for` or a parameter: what each
	/// element reads and binds ([`PatternInfo`]).
	pub patterns: FxHashMap<NodeId, PatternInfo>,
	/// The relations declared in the file (advanced brief §8), indexed by
	/// the `u32` of the tables below.
	pub relations: Vec<Relation>,
	/// The relation sides: by (type declaration node, field name), the
	/// relation and the side.
	pub relation_fields: FxHashMap<(NodeId, Sym), (u32, RelationSide)>,
	/// Every write of a direct side (an assignment `x.f = t`, a call
	/// `x.f.add(t)` / `delete(t)` / `clear()` of a many-to-many side, a
	/// literal given to `add` that sets direct sides): the relation(s).
	pub relation_writes: FxHashMap<NodeId, Vec<u32>>,
	/// Every removal from a collection that relations hold or target (calls
	/// of `delete`, `take`, `deleteWhere`, `clear`): the relations whose
	/// links the removed elements leave.
	pub relation_unlinks: FxHashMap<NodeId, Vec<u32>>,

	// ── compile-time evaluation (advanced brief §12, `comptime/`) ──
	/// The nodes whose value is known at compile time, with that value: an
	/// `@` call, the read of a `static`, `target`, a `static` declaration.
	/// A backend emits the value instead of the node ([`Analysis::constant`]).
	pub constants: FxHashMap<NodeId, fa_interpreter::Value>,
	/// The value of each `static` of the file.
	pub static_values: FxHashMap<SymbolId, fa_interpreter::Value>,
	/// The module-level `let` values (and `export =` values) whose value is
	/// a constant expression, with that value ([`Analysis::module_constant`]).
	pub module_constants: FxHashMap<SymbolId, fa_interpreter::Value>,
	/// Per `@if` node, the branch kept: `true` for the first, `false` for
	/// the `@else` (absent when there is none). The other one is parsed but
	/// not analyzed (D22): its nodes have no type.
	pub comptime_branches: FxHashMap<NodeId, bool>,
	/// Per `@for` node whose items are known, the items: the body is
	/// emitted once per item, the loop variable bound to it.
	pub unrolled: FxHashMap<NodeId, Vec<fa_interpreter::Value>>,
	/// Per `@for` whose body reads its variable at compile time (an `@if`
	/// or an `@` call on it), analyzed once per item: the kept branches and
	/// the compile-time values of its body for each item, in item order.
	/// While emitting an item, a backend reads them before
	/// `comptime_branches` and `constants`.
	pub unrolled_items: FxHashMap<NodeId, Vec<ItemResults>>,
	/// While the walk analyzes an item of such an `@for`: the value of its
	/// variable, by the variable's declaration node (compile-time code reads it).
	pub comptime_locals: FxHashMap<NodeId, fa_interpreter::Value>,
	/// The files and folders compile-time code read (`@readFile`,
	/// `@readDirectory`), relative to the root: inputs of the build.
	pub build_inputs: Vec<String>,
	/// The analysis read `target` or `configuration`: it depends on the
	/// target (D22).
	pub reads_target: bool,
	/// The bodies of the work given to `WorkerPool.submit` (a lambda, or
	/// the name of a function): what they use is published for the
	/// program's worker rule (`CONC008`, `Interface::worker_work`).
	pub submitted_works: Vec<NodeId>,
	/// The calls of an operation of a parallel view
	/// (`numbers.parallel().map(f)`), with its name: `map`, `filter`,
	/// `reduce`, `some`, `every` or `sum` (native: `lower/parallel.rs`).
	pub parallel_operations: FxHashMap<NodeId, &'static str>,

	// ── reactivity (advanced brief §11, `check/reactivity.rs`) ──
	/// Every read of a reactive place that may run while a `derived` value
	/// or a `watch` block is computed: the source to track, by the node of
	/// the read (the outermost place expression: `numbers.length`, not
	/// `numbers`).
	pub reactive_reads: FxHashMap<NodeId, ReactivePlace>,
	/// Every write of a reactive place: the sources to notify after it, by
	/// the assignment, compound assignment or call node.
	pub reactive_writes: FxHashMap<NodeId, Vec<ReactivePlace>>,
	/// Per call of a function of this file that may receive a reactive
	/// place: per parameter (`FnInfo::params` order, `self` included), the
	/// reactive place its argument designates (D24).
	pub reactive_args: FxHashMap<NodeId, Vec<Option<ReactivePlace>>>,
	/// The statements of the outermost running code (top level, test body,
	/// entry function) after which the pending `watch` blocks run.
	pub flush_after: rustc_hash::FxHashSet<NodeId>,
	/// Every `watch` node: its cleanup and whether its value is kept.
	pub watches: FxHashMap<NodeId, WatchInfo>,
	/// The `@extern` functions of the file (foreign code): the C symbol of
	/// each `function name: Type = @extern("symbol")` node.
	pub externs: FxHashMap<NodeId, String>,
	/// The calls of `@extern` functions (of this file or another): the
	/// declaration called. Its `CallInfo` has `Callee::Value` and positional
	/// arguments.
	pub extern_calls: FxHashMap<NodeId, ExternCall>,
	/// The arguments converted for C at a call (foreign code, advanced
	/// brief §13.3), by value node. An argument absent here is passed as is.
	pub foreign_args: FxHashMap<NodeId, ForeignArg>,

	// ── async (`design/async.md` §2.2, `check/tasks.rs`) ──
	/// Per function declaration, lambda, `start do`, test and top level
	/// (`Module` node): its summary.
	pub async_summaries: FxHashMap<NodeId, AsyncSummary>,
	/// Per `await` node: what it awaits.
	pub awaits: FxHashMap<NodeId, AwaitInfo>,
	/// Per `start` node: the task it creates.
	pub starts: FxHashMap<NodeId, StartInfo>,
}

bitflags! {
	/// A set of targets (`design/async.md` §2.1): where the synchronous
	/// instance of a function can exist.
	#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
	pub struct TargetSet: u8 {
		/// `web`
		const WEB = 1 << 0;
		/// `bun`
		const BUN = 1 << 1;
		/// `node`
		const NODE = 1 << 2;
		/// `native` (and `native-freestanding`)
		const NATIVE = 1 << 3;
		/// `wasm`
		const WASM = 1 << 4;
	}
}

impl TargetSet {
	/// The target named `name` (`"web"`, `"native"`…); empty for an unknown name.
	pub fn named(name: &str) -> TargetSet {
		match name {
			"web" => TargetSet::WEB,
			"bun" => TargetSet::BUN,
			"node" => TargetSet::NODE,
			"native" | "native-freestanding" => TargetSet::NATIVE,
			"wasm" => TargetSet::WASM,
			_ => TargetSet::empty(),
		}
	}

	/// The names of the targets of the set, in a fixed order.
	pub fn names(self) -> Vec<&'static str> {
		[
			(TargetSet::NATIVE, "native"),
			(TargetSet::WASM, "wasm"),
			(TargetSet::WEB, "web"),
			(TargetSet::BUN, "bun"),
			(TargetSet::NODE, "node"),
		]
		.into_iter()
		.filter(|(target, _)| self.contains(*target))
		.map(|(_, name)| name)
		.collect()
	}
}

/// The async summary of a function (`design/async.md` §2.1): transitive
/// through its callees, recursive cycles settled at the end of the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AsyncSummary {
	/// It may suspend when awaited (A4): declared `async`, and it awaits
	/// something that may suspend. False for a function that isn't `async`.
	pub suspends: bool,
	/// A `start` (or `start do`) is reachable from its body.
	pub starts: bool,
	/// The targets where its synchronous instance can exist.
	pub blocking: TargetSet,
	/// Why targets are missing from `blocking`: the targets removed, and
	/// the call chain down to the primitive (`load → lock (…)`). Not part
	/// of the interface hash.
	pub blocked_by: Vec<(TargetSet, String)>,
	/// Why `suspends` or `starts` holds (`awaits Mutex.lock`), for `fa
	/// build -v`. Not part of the interface hash.
	pub reason: Option<String>,
}

impl Default for AsyncSummary {
	fn default() -> Self {
		Self {
			suspends: false,
			starts: false,
			blocking: TargetSet::all(),
			blocked_by: Vec::new(),
			reason: None,
		}
	}
}

impl AsyncSummary {
	/// The chain that makes a blocking call impossible on `target`.
	pub fn chain(&self, target: TargetSet) -> Option<&str> {
		self.blocked_by
			.iter()
			.find(|(targets, _)| targets.intersects(target))
			.map(|(_, chain)| chain.as_str())
	}
}

/// What an `await` awaits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AwaitKind {
	/// A call of an `async` function declaration (the operand): the backend
	/// chooses the callee's instance (the selection rule).
	Call,
	/// A `Task` value (the operand), consumed by the `await`.
	Task,
	/// A call through a value of an `async` function type.
	FunctionValue,
	/// A call of a primitive that waits, by name: `Mutex#lock`,
	/// `Channel#send`, `Channel#receive`, `Task.yield`, `sleep`, `worker`
	/// (a method of a worker).
	Primitive(&'static str),
}

/// An `await` node (`design/async.md` §2.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AwaitInfo {
	/// What it awaits.
	pub kind: AwaitKind,
	/// Whether it may suspend (A4): false for a call of an `async`
	/// function that can't suspend (a plain call).
	pub suspends: bool,
	/// `Cancelled` was added to its errors (A11).
	pub cancelled: bool,
	/// The operand is a task-returning expression awaited right away
	/// (`await start f()`, `await Task.all(…)`): a direct call of the async
	/// instance, without a queue round-trip (A15).
	pub immediate_task: bool,
}

/// A `start` node (`design/async.md` §2.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartInfo {
	/// A `start` whose task is not kept (a statement): it runs to its end,
	/// and the program waits for it before exiting.
	pub detached: bool,
	/// The `do` block of a `start do`; `None` for a started call (the operand).
	pub block: Option<NodeId>,
	/// The locals a `start do` uses: copied when it is detached, borrowed
	/// until the task ends otherwise.
	pub captures: Vec<SymbolId>,
	/// For a detached task: the argument values and the captured locals it
	/// copies (a name node, or the capture's declaration node), each with
	/// whether it is moved instead (its last use, A12).
	pub copies: Vec<(NodeId, bool)>,
	/// The scope whose end drops a kept task (cancels it and waits for it).
	pub scope: Option<ScopeId>,
}

/// A call of an `@extern` function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternCall {
	/// The file of the declaration.
	pub file: FileId,
	/// The `function` node.
	pub decl: NodeId,
	/// The C symbol.
	pub symbol: String,
}

/// How a Fa value is given to a C parameter (advanced brief §13.3).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ForeignArg {
	/// An `Array(T)` for a `Pointer(T)`: a pointer to its first item, valid
	/// for the call. Written back when the parameter is `mutates`.
	Array { mutated: bool },
	/// A `T` for a `Pointer(T)`: its address. A `mutates` parameter receives
	/// the address of the `mutable` place itself; otherwise the address of
	/// a copy.
	Address { mutated: bool },
	/// A `String` for a `CString`: a zero-terminated copy, for the call.
	CString,
	/// A `function` known at compile time, capturing nothing, for a function
	/// pointer parameter: the C function pointer of its declaration.
	Callback,
}

/// A reactive place, as the backends instrument it (advanced brief §11.6):
/// the source of a root, and the path below it. A read tracks the source
/// at the end of the path (a child source created on demand), a write
/// notifies it (which changes the sources below it and the whole values
/// above it: the length is a child that only `Length`/`Size` steps reach).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactivePlace {
	/// Where the source is.
	pub root: ReactiveRoot,
	/// The steps from the root's source, each with what computes it.
	pub steps: Vec<ReactiveStep>,
}

/// The root source of a [`ReactivePlace`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReactiveRoot {
	/// A `reactive` value (local or module-level): its own source
	/// (`count$s` in JavaScript).
	Value(SymbolId),
	/// A parameter of a non-flat type (or `self`): reactive only in the
	/// instances of its function that receive a reactive place for it, with
	/// its source as a hidden argument (D24). The symbol is the parameter's.
	Param(SymbolId),
	/// A `reactive` field: the field at index `member` of the members of the
	/// `Access` node `access`; the object is the access's target with the
	/// members before it (evaluate the target once). Its source belongs to
	/// the object (created on demand, never copied with it).
	Field {
		/// The `Access` node (for a method call, the callee).
		access: NodeId,
		/// The index of the reactive field in its members.
		member: u16,
	},
}

/// A step of a [`ReactivePlace`] below its root.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReactiveStep {
	/// A field.
	Field(Sym),
	/// An element or a map entry: the index or key node (evaluate it once,
	/// for both the access and the source).
	Index(NodeId),
	/// `.length` of an array or a string.
	Length,
	/// `.size` of a set, a map or a pool.
	Size,
}

/// A `watch` block (advanced brief §11.3).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WatchInfo {
	/// The body (a `Block`).
	pub body: NodeId,
	/// The cleanup: the lambda that is the last statement of the body.
	pub cleanup: Option<NodeId>,
	/// The value is used (`let subscription = watch { … }`): a
	/// `Subscription` that `stop()` disposes. Otherwise the block lives to
	/// the end of its scope (the program, at the top level).
	pub subscription: bool,
}

/// The compile-time results of the body of an `@for` for one of its items
/// ([`Analysis::unrolled_items`]).
#[derive(Clone, Debug, Default)]
pub struct ItemResults {
	/// The branch kept by each `@if` of the body.
	pub branches: FxHashMap<NodeId, bool>,
	/// The value of each `@` call of the body.
	pub constants: FxHashMap<NodeId, fa_interpreter::Value>,
}

/// A relation (advanced brief §8.2): a direct side `direct` on the elements
/// of the holder collection, pointing into the target collection, mirrored
/// by the reverse side `reverse` on the elements of the target collection.
#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
	/// What a write does (§8.5).
	pub kind: RelationKind,
	/// The type declaration whose elements carry the direct side.
	pub holder_type: NodeId,
	/// The direct side (`bestFriend`, `friends`, `spouse`, an owner field).
	pub direct: Sym,
	/// The type declaration of the targets.
	pub target_type: NodeId,
	/// The reverse side, on the targets (`bestFriendOf`); the direct side
	/// itself for a symmetric relation; `None` for an owner field.
	pub reverse: Option<Sym>,
	/// The collection whose elements hold the direct side (for an owner
	/// field: the collection it points into, owned by `holder_type`).
	pub holder: CollectionRef,
	/// The collection of the targets.
	pub target: CollectionRef,
}

/// The kind of a relation (advanced brief §8.2).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RelationKind {
	/// `f: (T in c)?` with `r: Pool(H->f)`.
	OneToMany,
	/// `f: (T in c)?` with `r: (H->f)?`.
	OneToOne,
	/// `f: Pool(T in c)` with `r: Pool(in H->f)`.
	ManyToMany,
	/// `s: (c->s)?`: its own reverse side.
	Symmetric,
	/// `g: (T in c)?` on the type that owns `c`: no reverse side, cleared
	/// when its target is deleted (§8.7).
	OwnerField,
}

/// Which side of a relation a field is.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RelationSide {
	/// The written side (a symmetric field is direct).
	Direct,
	/// The mirrored, read-only side.
	Reverse,
	/// An owner field.
	Owner,
}

/// A collection named by a relation: a field of an owner type (`people` of
/// `Town`: instantiated with the owner value at each use) or a module-level
/// value.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CollectionRef {
	/// A field of the type declared by `owner`.
	Field { owner: NodeId, name: Sym },
	/// A module-level value, by its declaration node.
	Module { declaration: NodeId },
}

/// A destructuring pattern (values brief §16): `let { a, b as c } = o`,
/// `let [first, second] = items`, `let (x, y) = pair`, nested patterns,
/// `for (key, value) in …`, `({ x, y }: Point) => …`.
///
/// The value is evaluated once; the elements are read in order. An
/// `ArrayPattern` whose length isn't proven has `INDEX_CHECK` and `CAN_FAIL`
/// (`IndexError { index = items - 1, length }` after one length check).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternInfo {
	/// The type of the destructured value.
	pub ty: TypeId,
	/// The elements, in source order: the fields of an object pattern, the
	/// items of an array or tuple pattern (their index is their position).
	pub elements: Vec<PatternElement>,
	/// `...rest`: its symbol (`decl_node`: the rest name node) and its type
	/// (an object type without the named fields, or an `Array` of the rest).
	pub rest: Option<(SymbolId, TypeId)>,
}

/// One element of a destructuring pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternElement {
	/// The field read (object patterns), `None` for an item (by position).
	pub field: Option<Sym>,
	/// What the element binds.
	pub binding: PatternBinding,
	/// The type of the element as read from the value (the field type, the
	/// item type); `UNKNOWN` for a field absent from the value's type.
	pub ty: TypeId,
	/// The default value (`d = 4`): used when the field holds `none`, or always
	/// when the field is absent from the type (`absent`).
	pub default: Option<NodeId>,
	/// The field is not in the value's type: the binding is the default.
	pub absent: bool,
}

/// What an element of a pattern binds.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PatternBinding {
	/// A name: its symbol (for an object pattern field, `decl_node` is the
	/// pattern node; for an item, the name node).
	Symbol(SymbolId),
	/// A nested pattern node, itself in [`Analysis::patterns`].
	Nested(NodeId),
	/// Nothing (an invalid element, already reported).
	None,
}

/// A field of an object type, as declared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDecl {
	/// The field node (`x: Integer = 0`, `readonly y: T`).
	pub node: NodeId,
	/// The name (`#secret` for a private field).
	pub name: Sym,
	/// The type.
	pub ty: TypeId,
	/// The default value, evaluated at each creation that omits the field.
	pub default: Option<NodeId>,
	/// The default value of a field of a type declared in another file,
	/// evaluated there at compile time: an index into
	/// [`Analysis::field_constants`].
	pub constant: Option<u32>,
	/// `readonly`, `#private`.
	pub flags: FieldFlags,
}

/// A `type` declaration (design §2.6): what the backends need to build and
/// print its values, and to call its functions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDecl {
	/// Its symbol.
	pub symbol: SymbolId,
	/// The declared type: an object type, a named union, an enumeration, or
	/// the aliased type.
	pub ty: TypeId,
	/// The fields of an object type, in declaration order (the order of the
	/// properties of its JavaScript objects, `design/decisions.md`).
	pub fields: Vec<FieldDecl>,
	/// The functions declared in the type: methods (first parameter `self`)
	/// and associated functions, in declaration order.
	pub functions: Vec<SymbolId>,
	/// The scope of its functions, when it has a body.
	pub scope: Option<ScopeId>,
	/// Its generic parameters (`type Point(NumberType) = …`), in order: an
	/// application `Point(Float)` is the type substituted.
	pub generics: Vec<ParamId>,
}

/// What fills a field of an object literal.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FieldValue {
	/// The value written in the literal.
	Given(NodeId),
	/// Omitted: the default value of the field's declaration.
	Default(NodeId),
	/// Omitted, a field of a type of another file whose default value is
	/// known at compile time: [`Analysis::field_constants`] at this index,
	/// emitted like a compile-time value (a fresh value at each creation).
	Constant(u32),
	/// Omitted, optional without default: `none`.
	None,
	/// Copied from a spread `...value` of the literal (`value` is the spread's
	/// value node, evaluated once, in source order with the written fields).
	Spread(NodeId),
}

/// An object literal (design §2.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectInfo {
	/// The object type built (for a named-union member, its data type).
	pub ty: TypeId,
	/// The declaration it is built from, when there is one.
	pub decl: Option<NodeId>,
	/// The member of a named union, when built as `Shape.Circle { … }`.
	pub member: Option<TypeId>,
	/// Every field, in declaration order (literal order for an anonymous type).
	pub fields: Vec<(Sym, FieldValue)>,
}

/// The stored form of [`ArithInfo`]: its interval is interned.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ArithEntry {
	/// The values the node may take (`Analysis::interval_values`).
	pub interval: IntervalId,
	/// The type the tree is computed in.
	pub compute: ComputeType,
	/// The root of its arithmetic tree.
	pub root: bool,
}

/// The arithmetic facts of an integer node, for the backends (design §3.6),
/// as [`Analysis::arith`] gives them.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ArithInfo {
	/// The values the node may take, from the stable loop round. Kept by
	/// value: the fact store of the walk doesn't outlive it.
	pub interval: Interval,
	/// The type the tree is computed in (values brief §6.7).
	pub compute: ComputeType,
	/// The root of its arithmetic tree: its parent is not an arithmetic node
	/// of the same tree.
	pub root: bool,
}

/// What the backends need of a function or a lambda node.
#[derive(Clone, Debug, PartialEq)]
pub struct FnInfo {
	/// The function type: declared, or inferred for a single-expression body;
	/// `UNKNOWN` when its signature couldn't be resolved.
	pub ty: TypeId,
	/// Its signature: parameter types, return type (declared, or inferred from
	/// a single-expression body, or `Void` for a block without return type),
	/// `throws` (`NEVER`: none), effects (empty in M1). `None` with `UNKNOWN`.
	pub sig: Option<SigId>,
	/// The symbol of a named function (`None` for a lambda).
	pub symbol: Option<SymbolId>,
	/// The `function name = …` node, `None` for a lambda.
	pub declaration: Option<NodeId>,
	/// The lambda node (`ArrowFunction`) holding the signature and the body.
	pub lambda: NodeId,
	/// The parameters, in order.
	pub params: Vec<FnParam>,
	/// The body: a `Block` node, or an expression.
	pub body: NodeId,
	/// Whether the body is a block (its value is its last expression, or `return`).
	pub block_body: bool,
	/// Whether the return type is written in the source.
	pub declared_return: bool,
	/// The scope of the parameters, shared with the top-level declarations of a block body.
	pub scope: ScopeId,
	/// The errors the body may throw: the declared ones, or the inferred ones
	/// for an entry function, a test or a lambda (`NEVER` for none).
	pub throws: TypeId,
	/// The local symbols of enclosing functions it reads or writes, in order
	/// of first use: by reference while it doesn't escape, copied at its
	/// creation when it does (`Analysis::escapes`). Module-level values are
	/// never captured.
	pub captures: Vec<SymbolId>,
	/// Its generic parameters (`function first(Item) = …`), in order: the
	/// `TypeKind::Param` its signature uses. A call records the types they
	/// are bound to in `instantiations`, in this order.
	pub generics: Vec<ParamId>,
}

/// A parameter of a function or lambda.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnParam {
	/// The parameter node (`a: Integer = 1`, `...values: Integer`, `x`).
	pub node: NodeId,
	/// Its symbol (kind `Parameter`); its `ty` is the type seen by the body
	/// (`Array(T)` for a variadic parameter).
	pub symbol: SymbolId,
	/// The default value, evaluated in the callee at each call that omits the
	/// argument, after the previous parameters are bound (it can read them).
	pub default: Option<NodeId>,
	/// `...values: T`: collects the remaining positional arguments into an `Array(T)`.
	pub variadic: bool,
}

/// What a call calls.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Callee {
	/// A function declaration of this file (a direct call): the `function` node.
	/// Named arguments and defaults only exist for these.
	Function(NodeId),
	/// A built-in function of the prelude (`console.log`).
	Builtin(BuiltinId),
	/// A conversion to a number type (`Int8(x)`, `Float(n)`): the target type.
	Conversion(TypeId),
	/// Any other value of a function type (positional arguments only).
	Value,
	/// A member of a named union built with its data, `Shape.Labelled("north")`:
	/// the member type; the one argument is the data.
	Member(TypeId),
}

/// What fills one parameter at a call.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Argument {
	/// The value node of the argument (for a named argument, its value, not the
	/// `name = value` node).
	Given(NodeId),
	/// Not given: the callee evaluates the parameter's default value.
	Default,
	/// The positional arguments collected by a variadic parameter, in order: a
	/// `Spread` node (`...numbers`) spreads a collection.
	Variadic(Vec<NodeId>),
	/// Missing (already reported).
	Missing,
	/// The receiver of a method call `value.method(…)` (the `self`
	/// parameter): the callee `Access` node without its last member.
	Receiver(NodeId),
}

/// What provides a requirement at a call (advanced brief §9.3, §9.7).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RequirementArg {
	/// The value of this `provide` statement: an alias of its place when the
	/// value is a place (native: its address; JavaScript: the value), else
	/// the temporary it evaluated once, owned by its block.
	Provided(NodeId),
	/// The enclosing function's own requirement of this key: its hidden
	/// parameter at the key's position in its `FnSig::requires`.
	Own(TypeId),
	/// Not provided (reported, `REQ001`).
	Missing,
}

/// A call (design §2.6): its callee and its arguments in parameter order.
#[derive(Clone, Debug)]
pub struct CallInfo {
	/// What is called.
	pub callee: Callee,
	/// One entry per parameter of the callee, in parameter order.
	pub arguments: Vec<Argument>,
}

impl Analysis {
	/// An empty analysis of a file with `nodes` nodes.
	pub fn new(file: FileId, nodes: usize) -> Self {
		Self {
			file,
			types: vec![TypeId::UNSET; nodes],
			resolutions: vec![Resolution::Unresolved; nodes],
			flags: vec![NodeFlags::empty(); nodes],
			symbols: SymbolTable::new(),
			diagnostics: Vec::new(),
			arith: FxHashMap::default(),
			coercions: FxHashMap::default(),
			intervals: FxHashMap::default(),
			interval_values: Interner::new(),
			instantiations: FxHashMap::default(),
			functions: FxHashMap::default(),
			calls: FxHashMap::default(),
			escapes: rustc_hash::FxHashSet::default(),
			non_escaping: rustc_hash::FxHashSet::default(),
			requirement_args: FxHashMap::default(),
			type_decls: FxHashMap::default(),
			objects: FxHashMap::default(),
			field_constants: Vec::new(),
			json_defaults: FxHashMap::default(),
			moved_reads: rustc_hash::FxHashSet::default(),
			patterns: FxHashMap::default(),
			relations: Vec::new(),
			relation_fields: FxHashMap::default(),
			relation_writes: FxHashMap::default(),
			relation_unlinks: FxHashMap::default(),
			constants: FxHashMap::default(),
			static_values: FxHashMap::default(),
			module_constants: FxHashMap::default(),
			comptime_branches: FxHashMap::default(),
			unrolled: FxHashMap::default(),
			unrolled_items: FxHashMap::default(),
			comptime_locals: FxHashMap::default(),
			build_inputs: Vec::new(),
			reads_target: false,
			submitted_works: Vec::new(),
			parallel_operations: FxHashMap::default(),
			reactive_reads: FxHashMap::default(),
			reactive_writes: FxHashMap::default(),
			reactive_args: FxHashMap::default(),
			flush_after: rustc_hash::FxHashSet::default(),
			watches: FxHashMap::default(),
			externs: FxHashMap::default(),
			extern_calls: FxHashMap::default(),
			foreign_args: FxHashMap::default(),
			async_summaries: FxHashMap::default(),
			awaits: FxHashMap::default(),
			starts: FxHashMap::default(),
		}
	}

	/// The compile-time value of a node, to emit instead of the node.
	pub fn constant(&self, node: NodeId) -> Option<&fa_interpreter::Value> {
		self.constants.get(&node)
	}

	/// Whether the value of a local (a `let` or `mutable` of a block or a
	/// function) never outlives its function's frame: it is never returned,
	/// thrown, stored into a field, an element, a collection (`push`,
	/// `add`…) or a module-level value, moved into a detached task, nor
	/// moved into a local that escapes (`check/locals.rs`). A backend may
	/// keep it in the frame (natively: a string of up to 64 bytes in a stack
	/// buffer). Reads of it as an argument of a user function are fine:
	/// parameters are references, and a callee that keeps one copies it.
	pub fn is_non_escaping(&self, symbol: SymbolId) -> bool {
		self.non_escaping.contains(&symbol)
	}

	/// The value of a module-level `let` whose value is a constant
	/// expression (minimal representations): a backend may fold its reads
	/// or store it once in read-only data, and never reload it in a loop.
	/// Unlike [`Analysis::constant`], the declaration and its reads are still
	/// valid code: the JavaScript backend keeps `const name = value`.
	pub fn module_constant(&self, symbol: SymbolId) -> Option<&fa_interpreter::Value> {
		self.module_constants.get(&symbol)
	}

	/// The branch of an `@if` that is kept: `Some(true)` the first,
	/// `Some(false)` the `@else`, `None` when it was not decided.
	pub fn comptime_branch(&self, node: NodeId) -> Option<bool> {
		self.comptime_branches.get(&node).copied()
	}

	/// A destructuring pattern: what its elements read and bind.
	pub fn pattern(&self, node: NodeId) -> Option<&PatternInfo> {
		self.patterns.get(&node)
	}

	/// The type of a node.
	pub fn type_of(&self, node: NodeId) -> TypeId {
		self.types
			.get(node.index())
			.copied()
			.unwrap_or(TypeId::UNSET)
	}

	/// What a name or member node refers to.
	pub fn resolution(&self, node: NodeId) -> Resolution {
		self.resolutions
			.get(node.index())
			.copied()
			.unwrap_or_default()
	}

	/// The flags of a node.
	pub fn flags_of(&self, node: NodeId) -> NodeFlags {
		self.flags.get(node.index()).copied().unwrap_or_default()
	}

	/// Whether a node has all these flags.
	pub fn has_flags(&self, node: NodeId, flags: NodeFlags) -> bool {
		self.flags_of(node).contains(flags)
	}

	/// The symbols and scopes of the file.
	pub fn symbols(&self) -> &SymbolTable {
		&self.symbols
	}

	/// The local symbol a name refers to (`None` for the prelude, other
	/// modules, fields…).
	pub fn symbol_of(&self, node: NodeId) -> Option<&Symbol> {
		match self.resolution(node) {
			Resolution::Local(symbol) => Some(self.symbols.symbol(symbol)),
			_ => None,
		}
	}

	/// The symbol a declaration node declares (`let`, `function`, `type`…).
	pub fn declared_by(&self, node: NodeId) -> Option<&Symbol> {
		self.symbols
			.symbol_of_decl(node)
			.map(|symbol| self.symbols.symbol(symbol))
	}

	/// The arithmetic facts of an integer arithmetic node or range-checked sink.
	pub fn arith(&self, node: NodeId) -> Option<ArithInfo> {
		let entry = self.arith.get(&node)?;
		Some(ArithInfo {
			interval: self.interval_value(entry.interval),
			compute: entry.compute,
			root: entry.root,
		})
	}

	/// An interval of `interval_values`.
	pub fn interval_value(&self, id: IntervalId) -> Interval {
		*self.interval_values.resolve(id.0)
	}

	/// Interns an interval into `interval_values`.
	pub(crate) fn interval_id(&mut self, interval: Interval) -> IntervalId {
		IntervalId(self.interval_values.intern(interval))
	}

	/// Records the arithmetic facts of a node.
	pub(crate) fn set_arith(&mut self, node: NodeId, info: ArithInfo) {
		let interval = self.interval_id(info.interval);
		self.arith.insert(
			node,
			ArithEntry {
				interval,
				compute: info.compute,
				root: info.root,
			},
		);
	}

	/// Records the interval of an integer node.
	pub(crate) fn set_interval(&mut self, node: NodeId, interval: Interval) {
		let id = self.interval_id(interval);
		self.intervals.insert(node, id);
	}

	/// The implicit conversion of a value at its sink: the type it is converted to.
	pub fn coercion(&self, node: NodeId) -> Option<TypeId> {
		self.coercions.get(&node).copied()
	}

	/// The interval of an integer node, when narrower than its type.
	pub fn interval(&self, node: NodeId) -> Option<Interval> {
		self.intervals.get(&node).map(|&id| self.interval_value(id))
	}

	/// The type arguments of a generic call or literal (M2).
	pub fn instantiation(&self, node: NodeId) -> Option<ListId> {
		self.instantiations.get(&node).copied()
	}

	/// The information of a function declaration or lambda node.
	pub fn function(&self, node: NodeId) -> Option<&FnInfo> {
		self.functions.get(&node)
	}

	/// The `type` declaration at a node.
	pub fn type_decl(&self, node: NodeId) -> Option<&TypeDecl> {
		self.type_decls.get(&node)
	}

	/// The object literal at a node.
	pub fn object(&self, node: NodeId) -> Option<&ObjectInfo> {
		self.objects.get(&node)
	}

	/// The call at a node: its callee and its arguments in parameter order.
	pub fn call(&self, node: NodeId) -> Option<&CallInfo> {
		self.calls.get(&node)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flags_by_name() {
		assert_eq!(
			NodeFlags::from_name("SINK_CHECK"),
			Some(NodeFlags::SINK_CHECK)
		);
		assert_eq!(
			NodeFlags::from_name("OVERFLOW_NATIVE"),
			Some(NodeFlags::OVERFLOW_NATIVE)
		);
		assert_eq!(NodeFlags::from_name("NOPE"), None);
		assert!(NodeFlags::CHECKS.contains(NodeFlags::DIVISOR_CHECK));
		assert!(!NodeFlags::CHECKS.contains(NodeFlags::NARROWED));
	}

	#[test]
	fn out_of_range_nodes_have_no_type() {
		let analysis = Analysis::new(FileId(0), 2);
		assert_eq!(analysis.type_of(NodeId(5)), TypeId::UNSET);
		assert_eq!(analysis.flags_of(NodeId(1)), NodeFlags::empty());
		assert!(analysis.arith(NodeId(1)).is_none());
		assert!(analysis.coercion(NodeId(1)).is_none());
		assert!(analysis.function(NodeId(1)).is_none());
		assert!(analysis.symbol_of(NodeId(1)).is_none());
	}

	#[test]
	fn accessors_read_the_side_tables_of_a_walk() {
		let (session, file) = crate::analyze_source("let answer = 42\nconsole.log(answer)\n");
		let analysis = session.analysis(file);
		let tree = session.tree(file);
		let read = (0..tree.nodes.len())
			.rev()
			.find(|&node| tree.text(node) == "answer")
			.map(NodeId::new)
			.unwrap();
		let symbol = analysis.symbol_of(read).unwrap();
		assert_eq!(session.types().syms.resolve(symbol.name), "answer");
		assert!(analysis.declared_by(NodeId::new(tree.root)).is_none());
		let declared = (0..tree.nodes.len())
			.map(NodeId::new)
			.find(|&node| analysis.declared_by(node).is_some())
			.unwrap();
		assert_eq!(analysis.declared_by(declared).unwrap().ty, TypeId::INTEGER);
		assert_eq!(analysis.symbols().symbols().len(), 1);
		assert!(!analysis.has_flags(read, NodeFlags::TYPE_EXPR));
	}
}
