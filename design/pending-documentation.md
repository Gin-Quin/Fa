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

Make precise: `parallel()` gives a view for one operation, called at once on it: `map`, `filter`, `reduce`, `some`, `every` or `sum` (`let view = numbers.parallel()` alone is an error). `filter` keeps the collection's kind. The function of `reduce` combines two items (its accumulator is of the items' type), since the items are grouped in any way. A parallel view of a `Map` runs over its entries (`(key, value)`, in the map's order) and has `map`, `filter`, `some` and `every` (`map` and `filter` give a map with the same keys); `reduce` isn't available on it (use `Array.from(map.values()).parallel().reduce(…)`).

## `documentation/architecture/dependencies.md`, "Sibling packages" (and the table of "Sources")

Add after the paragraph on sibling packages: "A package outside a workspace can depend on a local folder that holds a package, with a relative path too (`shared = "../shared"`). Unlike a sibling package, it is locked like any other source: the lockfile records the path and the hash of its `package.fa`, `dependencies.fa` and `library/`, and a change of these files fails the build until `fa update shared` accepts it." In the table of sources, the row "Sibling package of the workspace" becomes "Local package (a sibling in a workspace, a folder outside one)".

## The rt-5 example (`@typeToString(getDigitCount)`, types-at-runtime or function composition page)

A function name is a value: write `@typeToString(Type<getDigitCount>)` (metaprogramming page: "use `Type(value)` to get the type of a value"; `TY008` otherwise).

## `documentation/guides/web-server.md`, "Typed request and response bodies" (JSON of named unions)

A member of a named union is read and written tagged by a `"type"` property holding its name (`{ "type": "Circle", "radius": 1 }`); a member without data is `{ "type": "Point" }`, data that isn't an object (or an object with its own `type` field) is under `"value"`, a literal member is the literal. An unknown `"type"` is a `JsonError` naming the accepted members.
## `documentation/guides/web-server.md`, "Typed request and response bodies" (JSON of named unions)

A member of a named union is read and written tagged by a `"type"` property holding its name (`{ "type": "Circle", "radius": 1 }`); a member without data is `{ "type": "Point" }`, data that isn't an object (or an object with its own `type` field) is under `"value"`, a literal member is the literal. An unknown `"type"` is a `JsonError` naming the accepted members.

## The rt-5 example (`@typeToString(getDigitCount)`, types-at-runtime or function composition page)

A function name is a value: write `@typeToString(Type<getDigitCount>)` (metaprogramming page: "use `Type(value)` to get the type of a value"; `TY008` otherwise).

## `documentation/advanced/relations.md`, "Copies and references" (the module-level form)

After "In the second form, there is one collection for the whole program, and every file can add people to it.", add: "Both sides of a relation are declared in the same file, the one that declares the types of the elements; the collection can be in another file. That file reads the relation's file for the type of its elements, and reports there a collection whose elements move or are of another type." The example's functions that write `people` from another file declare it (`mutates people`), as in "Exporting a mutable value" of the modules page.

## `documentation/advanced/relations.md`, "Effects" (reactivity)

Add a paragraph: "Implied effects count for reactivity too. A relation side of an element of a reactive collection is observed like a `reactive` field: a `derived` value or a `watch` block reading `bao.bestFriendOf` runs again when a write adds Bao to it or removes Bao from it (`ana.bestFriend = bao`, `ana.bestFriend = cleo` afterwards, deleting Ana), and not for the writes that leave it unchanged. Since relation fields are not data, these writes don't change the collection itself: reading `people.size` doesn't depend on them. A `watch` block that writes a direct side and reads its reverse side writes what it reads (compile error)." `:::implementation`: "each side of an object has its own source, created when an observer reads it; the two-sided update notifies the sources of the sides it changes (JavaScript: a callback given to the runtime helpers; native: the runtime's relation functions, the sources of sides keyed by their offset)."

## `documentation/advanced/relations.md`, `:::implementation` section (JavaScript, generic code)

Add to the JavaScript lowering: "An element of a type with relation sides carries a symbol-keyed property naming its sides, read by the generic copy, printing, equality and hashing of erased generic code."



## `documentation/advanced/foreign-code.md`, "Targets" and `:::implementation` ("JavaScript", "State of the implementation")

`bun` passes `CStruct`s by value now: drop "(a `CStruct` can't be passed by value)" from the table, and from the JavaScript state. Implementation: a C function taking or returning a `CStruct` by value goes through a shim that Bun's TinyCC compiles on its first call (`bun:ffi`'s `cc`): the arguments are written into memory with the C layout the compiler computes, the shim calls the function with them by value under the platform's C ABI, and writes the result back.

## `documentation/advanced/foreign-code.md`, "`CStruct`" (after the `Vertex` example)

Add: a `CArray(T, n)` value is written as an array literal of exactly `n` items (`position = [1.0, 2.0, 3.0]`); it is indexed like an array (`try vertex.color[i]`, from the end when negative), written in place (`vertex.position[1] = 5.5`), iterated (`for value, index in vertex.color`), and has a `length`. It is a value: copying the `CStruct` copies it.

## `documentation/advanced/foreign-code.md`, `:::implementation` ("Native", "State of the implementation")

`CStruct`s by value now follow the C ABI of the target: x86-64 System V, AAPCS64 (homogeneous aggregates of 1 to 4 floats or doubles in floating-point registers, other structs up to 16 bytes as integers, larger ones through a pointer to a copy, or `sret`) and wasm32 (a struct of one scalar is that scalar, others `byval`/`sret`). Windows x64 isn't classified yet. Replace "`CStruct` by value on x86-64 System V only" in the state.

## `documentation/advanced/foreign-code.md`, `:::implementation` ("State of the implementation")

The analyzer and the native backend (and wasm) support `Pointer.allocate(value)` and `pointer.free()`: a Fa value in such a block is read as a copy (`read`), written with the old value destroyed (`write`), and `pointer.value` is a view of it. Bun doesn't yet.

## `documentation/advanced/foreign-code.md`, "Targets" and `:::implementation` ("WebAssembly", "State of the implementation")

On `wasm`, an `@extern` function is an import of the `env` module unless C compiled to WebAssembly and linked with the program (`build.link`: `.o`/`.a` files built with `clang --target=wasm32`) defines it. The host gives the others: the loader imports them from `env.mjs`, next to it (`fa build` copies the `env.mjs` found next to the program's file or at the root of its package); `env.mjs` exports one function per C name, and may export `attach(exports)` to reach the module's memory. A missing function stops the loader with the list of names. Replace "WebAssembly: not yet" in the state.

## `documentation/types/fields.md`, "Field sets" and "String representation"

Add: `+=` and `-=` work on a `mutable` set (`permissions += Permissions.admin`); any other operator is an error. `Integer(set)` needs an integer type holding every value of the set (a set of 64 flags or more converts to `UInt64`/`UInt128`). A set prints as a key set: `console.log(MyFields { foo, nested { baz1 } })` prints `{ foo, nested { baz1 } }`, and `{}` when empty. A group may list no flag in a value (`nested {}`), not in a declaration.

## `documentation/types/fields.md`, "Key sets as compile-time parameters"

Make precise: the type parameter whose keys a key set holds comes before it (`<Value, keys: DeepKeys<Value>>`); for now the body uses the parameter only as the iterable of `@for`, and only applications from the parameter's own file are accepted. An error in the body for one key is `TF011`, reported once with the key that causes it and a note at the key set of the application.

## `documentation/types/fields.md`, a new section "JSON and `when`" (after "String representation")

A set is written in JSON as the array of the paths of its flags, like `toStringArray()`: `Json.stringify(MyFields { foo, nested { baz2 } })` is `["foo","nested.baz2"]`. `Json.parse` reads such an array, a group's name standing for all its flags (`["nested"]`); another name is a `JsonError`. In `when`, an arm whose pattern is a value of the set's type (`Permissions { read, write }`, `Permissions.read`, `{}`) matches an equal set; the other sets need an `else`.

## `documentation/advanced/foreign-code.md`, "Exposing a library to C"

Make precise (decided, see "C libraries" in `design/decisions.md`):

- `fa build --target=native` writes `libfarm.a`, `libfarm.so` and `farm.h` (named after the prefix) into `build/native/` of the package, and no executable. A C program links the static library with `-lm -pthread` and the libraries of `build.link`.
- The C types of the header: `int8_t`… `uint64_t` for the integers (`c.Int` is an `int32_t`), `bool`, `float`, `double`, `const char *` for a `CString`, `T *` for a `Pointer(T)` (`NULL` for `none`), a `CStruct` defined with its fields in order (`typedef struct farm_Point { int32_t x; float y; } farm_Point;`), a Fa value an opaque `struct` named `prefix_` and its type's name. `Integer`, callbacks and generic functions can't be exported (`FFI014`).
- A function that `throws` throws the members of one `Errors` type, declared as `typedef enum farm_FarmError { FARM_FARM_ERROR_EMPTY = 1, … } farm_FarmError;`; its result goes through the last parameter, `result`, which may be `NULL`. `farm_last_error_message()` returns `""` before the first error, and isn't reset by a success; a message longer than 1023 bytes is cut.
- A shared library is loaded at the program's start or with `dlopen` (Python's `ctypes`); only the API is visible. Its module values are dropped when it is unloaded if a `destroy` needs it. The output of `console.log` is flushed at the end of each call, so it keeps its order with C's.
- The example's `Farm.Error` is a type declared inside `Farm` (`type Error = Errors { … }`): show it, as the first example shows `Self.OpenError`. The header names it `farm_FarmError` (the path joined), its message reads `Error.Empty`.

## `documentation/advanced/foreign-code.md`, `:::implementation` ("Native", "State of the implementation")

- Native: add "A C library's thread-local variables use the general dynamic model (`dlopen` can't give the static TLS the runtime's `initial-exec` variables take), the shared library exports the API through a version script, and the static library is one relocatable object (`ld.lld -r`, then `llvm-objcopy --keep-global-symbol`). The error message goes to a thread-local buffer of 1024 bytes. `.init_array` and `.fini_array` hold the load and unload functions (`DllMain` on Windows is not done)."
- State: drop "and `build.cLibrary`" from the analyzer's gaps; add to the native state "`build.cLibrary` (Linux and macOS; not on `wasm` yet: the table's "yes: the functions are exports of the module" is not implemented)".

## `documentation/architecture/configuration.md`, "`build`" (`cLibrary`)

Add: "`cLibrary` applies to the `native` target (`FFI013` when the package also lists a JavaScript target: put it in `overrides.native.build`). `prefix` is a C identifier, `header` a file name; the libraries and the header are written into `<output>/native/`."

## `documentation/types/objects.md` (or the page on type declarations), "Types inside a type"

Document what foreign-code.md and relations.md use: a type can declare types in its body (`type Database = { type OpenError = Errors { … } }`). Inside the type they are named directly (`OpenError`) or `Self.OpenError`; outside, `Database.OpenError` (and `library.Database.OpenError` from another file), in types and in values: its members (`Database.OpenError.Failed { code }`), typed literals, `when` patterns, its functions (`Palette.Swatch.make(…)`), `members` of an enumeration. `Error` can name such a type (`Farm.Error`), though the word alone is the prelude's `Error`. A value prints with the type's own name (`Error.Empty`).
## `documentation/types/fields.md`, `:::implementation` section (native)

Add: "**Fields set once.** When no code of the program writes a field after the literal that builds its object (no assignment, append or changing reference of the field, no whole value of the type replaced but by a declaration), a method reads it once at its entry, a string with its bytes and length, and the small methods it calls on `self` (like `function current = (self): Byte => self.text.bytes().get(self.position) ?? 0`) are expanded into its body to use that read: a parser scanning `self.text` tests whether the text is inline once per call, not once per byte."

## `documentation/architecture/testing.md`, "How tests run"

Add a bullet: "A test file that is still running after `FA_TEST_TIMEOUT` seconds (300 by default) is stopped, and reported as failed with that reason: a test that never ends can't block `fa test`." (Note: the per-test `test.timeout` of the configuration is parsed but not enforced by the Bun runner yet; this per-file limit is the backstop.)

## `documentation/architecture/dependencies.md`, sources and lockfile (decision "Dependency security")

Add to the list of sources: "`file://` URLs, paths (`../shared`) and Git repositories on the disk (`git+file://…`, `git+/srv/x.git`) are allowed only in the root package and the packages of the workspace (and the path dependencies they name): a dependency fetched from Git or a remote source that declares or `@import`s one is `DEP008`. An `http://` source gets a `DEP004` warning: its first content is not authenticated." Add to the lockfile section: "A `hash` is `sha256:` followed by 64 lowercase hexadecimal digits and a `commit` 40 or 64 of them; anything else makes the lockfile invalid. The paths of a remote folder's `manifest.fa` (and of a Git tree) are relative paths with `/` separators, without `.`, `..`, empty segments, backslashes, colons or control characters (`DEP007` otherwise)." In "Unversioned URLs" (`DEP004`): only the path of a URL names a version (`/v3/`, `@2.1.0`, `-1.2`), never its host.

## `documentation/architecture/packages.md` (or the page of `fa check`)

Add: "With diagnostics in several files, the text report ends with one total line (`3 errors, 1 warning in 2 files`) instead of a summary per file. An error that stops the load for another target of the package (`DEP009`) is reported with `note: for the target \`web\``." And: "A standalone file reads only the files and folders next to it whose names its files mention, transitively."
## `documentation/collections/arrays.md`, `:::implementation` section (native)

Add: "**Packed arrays.** An array of a named union that the program only builds with `push` from an empty literal, reads with `for` and measures with `length` stores its items one after the other, each as its member and its data without padding (a string as its length and bytes), so that an item takes the bytes its member needs, with no box per item. **Large blocks.** Natively on Linux, the items of an array (and the bytes of a string) of 1 MiB or more have a mapping of their own, which grows by moving its pages rather than copying its bytes. **Arrays wrapped into values.** A small array built in a local, then stored into a field of a literal (`Sexp.List { items }`), moves its items next to its header, of their exact size." (See "Packed arrays" and the two entries after it in `design/decisions.md`.)

## `documentation/` page of recursive types (the arena section of its `:::implementation`, native)

Add: "**Compressed links.** On 64-bit Linux, the recursive edges of a type whose nodes live in arenas are 32-bit links (the node's address divided by 8): every arena page comes from one range reserved between 2 GiB and 32 GiB of the address space, so a `Tree` of two optional children takes 8 bytes. When that range is full, the program stops with `stop: out of memory (the range of the arenas of recursive values is full)` (exit code 101). wasm keeps its 32-bit pointers."

## `documentation/` page of `when`, `:::implementation` section (native)

Add: "A `when` choosing among string literals of one length by the integers `0` to `n - 1` and `else` reads the chosen one from a single constant holding them all, at `start + length × index` (any other integer reads the `else` literal): no branch and no table."
