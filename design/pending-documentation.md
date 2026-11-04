# Documentation notes waiting for the rewrite

The documentation is being rewritten outside this project (angle-bracket generics, 2026-11-04). Until it lands, engineers don't edit `documentation/`: notes meant for a page go here, with their page and section, and the lead applies them to the rewritten pages.

## `documentation/values/strings.md`, `:::implementation` section (native strings)

Replace the bullet "The parts of `split` are views…" with these two bullets:

- **The parts of `split` are views** of the text when the compiler proves that the text outlives them unchanged: the call (maybe followed by `filter`s) is the value of a `let`, the text is a `let` or `mutable` of the file that no reference designates, and nothing writes it in the scope of the parts nor in another function or closure. A view is a pointer and a byte length into the text, never freed; copying a part (pushing it, storing it in a map, assigning it) copies its bytes, and the array holding the views is never moved. The parts of an inline text are views of its copy in the frame of the function (not at the top level of a module file, whose initializer returns first). Otherwise each part is a copy (inline up to 23 bytes). A loop over the parts of a split by a non-empty literal knows that they are views, and reads them without testing their representation. A one-byte separator counts its occurrences first, so that the array of parts is allocated once.
- **Destination passing.** A function that builds its result in a `mutable` string and returns it, maybe followed by `+ …` (`mutable result = "\""` … `result + "\""`), called where its result is appended to a string (`out += quoted(name)`, a part of a template appended with `+=`), is expanded there with its accumulator standing for the destination: no string of its own. It must not throw, have effects, call functions of the program, return early or capture anything, and no argument may read the destination.

## `documentation/values/errors.md`, "Throwing errors" (or wherever `throws` in types is described)

Add: a value that throws fewer errors is accepted where a type throws more. A `Task(T)` is a `Task(T) throws E`, and a `(x: Integer) => Integer` is a `(x: Integer) => Integer throws E`: in a variable, an array, a field, an argument or a returned value. `:::implementation`: natively, a task's result always has room for an error (null when the task can't fail), so the task keeps its frame; a function value that throws nothing goes through an adapter (a closure holding a copy of it) where its type throws.

## `documentation/advanced/concurrency.md`, "3. Worker pools"

Add after "With a `timeout`, …": the task of `submit` also fails with the errors of the submitted function (`(worker) => try await worker.process(data)` gives a `Task(…) throws` its errors). A pool known to have no `timeout` (a local declared with a `WorkerPool(T) { … }` literal without `timeout`, never assigned another pool) gives tasks without `WorkerTimeout`; they are still accepted where a `Task(…) throws WorkerTimeout` is expected, as in the example.

## `documentation/types/types-at-runtime.md`, table of `Schema` members

No change: `signed: Boolean = true` of `Schema.Integer` is now what the prelude declares (built-in object types have field defaults).
