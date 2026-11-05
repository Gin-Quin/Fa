# Documentation notes waiting for the rewrite

The documentation is being rewritten outside this project (angle-bracket generics, 2026-11-04). Until it lands, engineers don't edit `documentation/`: notes meant for a page go here, with their page and section, and the lead applies them to the rewritten pages.

## `documentation/values/strings.md`, `:::implementation` section (native strings)

Replace the bullet "The parts of `split` are views…" with these two bullets:

- **The parts of `split` are views** of the text when the compiler proves that the text outlives them unchanged: the call (maybe followed by `filter`s) is the value of a `let`, the text is a `let` or `mutable` of the file that no reference designates, and nothing writes it in the scope of the parts nor in another function or closure. A view is a pointer and a byte length into the text, never freed; copying a part (pushing it, storing it in a map, assigning it) copies its bytes, and the array holding the views is never moved. The parts of an inline text are views of its copy in the frame of the function (not at the top level of a module file, whose initializer returns first). Otherwise each part is a copy (inline up to 23 bytes). A loop over the parts of a split by a non-empty literal knows that they are views, and reads them without testing their representation. A one-byte separator counts its occurrences first, so that the array of parts is allocated once.
- **Destination passing.** A function that builds its result in a `mutable` string and returns it, maybe followed by `+ …` (`mutable result = "\""` … `result + "\""`), called where its result is appended to a string (`out += quoted(name)`, a part of a template appended with `+=`), is expanded there with its accumulator standing for the destination: no string of its own. It must not throw, have effects, call functions of the program, return early or capture anything, and no argument may read the destination.

## `documentation/collections/maps.md`, `:::implementation` section (native maps)

Add after the description of the stable chunks: "When nothing in the program can keep a reference into the values of maps of a type (no `use`, `reuse` or `provide` of one, no value passed to a function of the program, none read in a lambda) and the program is compiled as one module, their entries are dense: one array that moves when it grows, so that a lookup reads the index slot and the entry, without the chunk directory (`design/decisions.md`, "Dense maps"). `GrowingMap` keeps its stable chunks."
## `documentation/values/errors.md`, "Throwing errors" (or wherever `throws` in types is described)

Add: a value that throws fewer errors is accepted where a type throws more. A `Task(T)` is a `Task(T) throws E`, and a `(x: Integer) => Integer` is a `(x: Integer) => Integer throws E`: in a variable, an array, a field, an argument or a returned value. `:::implementation`: natively, a task's result always has room for an error (null when the task can't fail), so the task keeps its frame; a function value that throws nothing goes through an adapter (a closure holding a copy of it) where its type throws.

## `documentation/advanced/concurrency.md`, "3. Worker pools"

Add after "With a `timeout`, …": the task of `submit` also fails with the errors of the submitted function (`(worker) => try await worker.process(data)` gives a `Task(…) throws` its errors). A pool known to have no `timeout` (a local declared with a `WorkerPool(T) { … }` literal without `timeout`, never assigned another pool) gives tasks without `WorkerTimeout`; they are still accepted where a `Task(…) throws WorkerTimeout` is expected, as in the example.

## `documentation/types/types-at-runtime.md`, table of `Schema` members

No change: `signed: Boolean = true` of `Schema.Integer` is now what the prelude declares (built-in object types have field defaults).

## `documentation/types/generics.md`, "Generic functions as values" (new, after "Explicit parameters")

Generic function values (phase 2 of "Generic functions carry their list on the function value"): a function type can be generic, `type Identity = <T>(value: T) => T`, and a parameter of that type is called at several types (`(transform: <T>(value: T) => T) => "{transform(1)} {transform("a")}"`). Values: a generic lambda `<T>(value: T) => value`, a generic `function` (`describe(identity)`), `function same: Identity = <T>(value: T): T => value`; stored in fields and collections, returned. Explicit arguments: `transform<String>("a")`. Given where a function type that isn't generic is expected, the value is instantiated by it (`[1, 2].map(<T>(value: T) => value)`). A function satisfies a generic function type with as many parameters when each constraint of the type satisfies the function's: `E-GEN-1` for a narrower constraint or another number of parameters, `E-GEN-2` for a function that isn't generic. `let f = <T>(…) => …` is still `E-DECL-6`: declare it with `function`.

## `documentation/types/generics.md`, `:::implementation` section (native generic function values)

A generic function value is the pair of any function value: a constant table of the instances of its type that the program calls (found by the whole-program compilation), and the environment of its lambda. No box; an instance set that keeps growing (a polymorphic recursion) is an error naming its first types. JavaScript: a plain function. See `design/llvm-backend-plan.md`, "Generic function values".

## `documentation/architecture/modules.md`, "Prelude" (the paragraph on `std`)

After "Some parts of `std` exist only on some targets: using one on a target that lacks it is a compile error.", add: `std.fs` doesn't exist on `web`, `std.net` on `web` and `wasm`; without a C library (`fa build --target=native-freestanding`, where `target` is `"native"`), neither do `std.fs`, `std.net` and `std.json`. `:::implementation`: natively, `Json.parse` reads the text into a tree, checks it against a schema of the expected type with the messages of the JavaScript runtime, then a reader generated per type builds the value; an integer may be any `Int64` (JavaScript stops at ±(2^53 − 1)). On `wasm`, the files go through the loader (`node:fs`), as the clock does.

## `documentation/advanced/concurrency.md`, table of what blocks per target

The WebAssembly column says "blocking (WASI)" for the file system and standard input: the target is not WASI (`design/decisions.md`, "WebAssembly target"); files go through the generated JavaScript loader's imports (`fileRun`/`fileTake`, `node:fs`), blocking. Write "blocking (through the loader)".
## `documentation/advanced/foreign-code.md`, the JavaScript target (around line 307)

Replace "`destroy` is called for a local at the end of its block when nothing moves it out; not yet for a resource inside another value or a collection, nor for the old value of a replaced local." by: `destroy` is called at the same points as natively: at the end of the block of the local or parameter owning the value (unless it moved out), for a resource inside another value or a collection, for the old value of a replaced local or field, and for a new value discarded; not yet for a resource of a type declared in another file. A resource may be all zero (a file descriptor 0): a move is tracked by a drop flag, never by its bytes (`design/decisions.md`, "A type holding a task is move-only").

## `documentation/types/types-at-runtime.md`, "The `Schema` type" and "`TypeFromSchema(Schema): Type`"

`@typeToSchema` of a named union or an enumeration (all its members) is a `Schema.Named` whose definition is the `Schema.NamedUnion` / `Schema.Enum`, like the other types used through a name; a `Schema.Function` names its parameters by position (`parameter1`, `parameter2`…: a function type keeps no names). `TypeFromSchema` builds every member but `Schema.Fields`: a `Schema.NamedUnion` / `Schema.Enum` gives a named union / enumeration named by its `Schema.Named` (else by the declaration applying it), a `Schema.Reference` to an enclosing `Schema.Named` a recursive type; an enumeration whose values are not the positions is not supported yet. As the value of a generic type function (`type Shouted<Input> = TypeFromSchema<@shout(@typeToSchema(Input))>`), the schema is evaluated per application; an error its code throws is `TF004` at the application, with the error's `message`.

## `documentation/guides/web-server.md`, "Typed request and response bodies"

A field missing from the JSON text takes its default value (a new one for each value read), even when it is optional; without a default, an optional field is `none` and another field is an error (`has no field`).

## `documentation/advanced/concurrency.md`, the table of targets and the `:::implementation` section (JavaScript)

Table row "`Mutex`, atomics shared with a worker": make it "`Mutex`, atomics, channels shared with a worker" (JavaScript: with `SharedArrayBuffer`, flat values only; on the web, the page must be cross-origin isolated, else giving one to a worker stops the program with the headers to send: `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: require-corp`). In the implementation section, replace "A `Channel` crossing a worker boundary becomes a `MessagePort`" by: a `Channel` given to a worker moves its queue into a `SharedArrayBuffer` (JSON text behind a lock word; a waiting task uses `Atomics.waitAsync` on a version word, a worker's blocking wait `Atomics.wait`), like a `Mutex`.

## `documentation/advanced/concurrency.md`, "1. Parallel loops" (the paragraph on `parallel()`)

Make precise: `parallel()` gives a view for one operation, called at once on it: `map`, `filter`, `reduce`, `some`, `every` or `sum` (`let view = numbers.parallel()` alone is an error). `filter` keeps the collection's kind. The function of `reduce` combines two items (its accumulator is of the items' type), since the items are grouped in any way. `parallel()` of a `Map` is not available yet.
## `documentation/architecture/dependencies.md`, "Sibling packages" (and the table of "Sources")

Add after the paragraph on sibling packages: "A package outside a workspace can depend on a local folder that holds a package, with a relative path too (`shared = "../shared"`). Unlike a sibling package, it is locked like any other source: the lockfile records the path and the hash of its `package.fa`, `dependencies.fa` and `library/`, and a change of these files fails the build until `fa update shared` accepts it." In the table of sources, the row "Sibling package of the workspace" becomes "Local package (a sibling in a workspace, a folder outside one)".

## The rt-5 example (`@typeToString(getDigitCount)`, types-at-runtime or function composition page)

A function name is a value: write `@typeToString(Type<getDigitCount>)` (metaprogramming page: "use `Type(value)` to get the type of a value"; `TY008` otherwise).
## `documentation/guides/web-server.md`, "Typed request and response bodies" (JSON of named unions)

A member of a named union is read and written tagged by a `"type"` property holding its name (`{ "type": "Circle", "radius": 1 }`); a member without data is `{ "type": "Point" }`, data that isn't an object (or an object with its own `type` field) is under `"value"`, a literal member is the literal. An unknown `"type"` is a `JsonError` naming the accepted members.
