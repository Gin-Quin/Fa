# Fa for C# developers

C# and Fa overlap more than most pairs: records, pattern matching, nullable reference types, `async`, LINQ-style chaining, even `throw`, `try` and `catch`. Fa takes each of those further, and drops classes, exceptions and the garbage collector.

Fa is designed for AI agents as much as for humans: everything a function does — the errors it throws, the services it requires, the data it mutates — is written in its signature, and if the code compiles, it works.

## Syntax at a glance

| C# | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `const`/`readonly var x = 1;` | `let x = 1` |
| `var x = 1;` | `mutable x = 1` |
| `int Add(int a) => a;` | `function add = (a: Integer): Integer => a` |
| `x => x + 1` | `(x) => x + 1` |
| `$"Hello {name}"` | `"Hello {name}"` |
| `long`, `double` | `Integer`, `Float` |
| `(double)count` | `Float(count)` |
| `List<T>` | `Array(T)` |
| `Dictionary<K, V>` | `Map(K, V)` |
| `HashSet<T>` | `Set(T)` |
| `T?` | `T?` |
| `null` | `none` |
| `x?.y` | `x?.y` |
| `x ?? y` | `x ?? y` (optionals only) |
| `switch` expression | `when x is { }` |
| `foreach (var x in xs)` | `for x in xs` |
| `x % y` | `x %% y` |
| `!x`, `&&`, `\|\|` | `not x`, `and`, `or` |
| `record Point(int X, int Y)` | `type Point = { x: Integer, y: Integer }` |
| `new Point(1, 2)` | `Point { x = 1, y = 2 }` |
| `int Load() { throw ...; }` | `function load = (): Integer throws LoadError => ...` |
| `try { ... } catch { ... }` | `try call()` to propagate, `call() catch error => ...` to handle |
| `Debug.Assert(x == y)` | `assert x == y` |
| `using var f = ...;` | `let f = ...` — every value is released at the end of its scope |
| `using System.Linq;` | nothing — files see their siblings |

## Records without the ceremony

```csharp
public record User(string Name, int Age = 0)
{
    public string Greet() => $"Hello, {Name}";
}
```

```fa
type User = {
	name: String
	age: Integer = 0

	function greet = (self): String => "Hello, {self.name}"
}

let user = User { name = "Ada", age = 30 }
let user = User { name = "Ada" } -- age is 0
```

Every type in Fa behaves like a record: `==` is a deep structural comparison, printing is structural, and values are copied on assignment. There is no `class` versus `struct` decision, because everything is a value.

Non-destructive mutation, C#'s `with` expression, is the spread operator:

```fa
let older = { ...user, age = user.age + 1 }
```

In Fa, `with` is something else: partial application (`format with (locale = "fr")`).

A method that changes its object declares it with `mutates self`:

```fa
type Counter = {
	value: Integer = 0

	function increment = (self) mutates self => {
		self.value += 1
	}
}
```

## No inheritance, no interfaces

Composition replaces inheritance:

```fa
type Admin = {
	...User
	permissions: Array(String)
}
```

And an interface is just a shape:

```fa
function process(Item: { id: Integer, name: String }) = (item: Item) => ...
```

Any type with those fields satisfies it, with nothing to declare. Fa is [structurally typed](/advanced/structural-typing) — this is C#'s implicit interface implementation, generalised to fields and applied everywhere.

## Errors are values, not exceptions

C# and Fa share the keywords `throw`, `try` and `catch`, but not their meaning. In Fa, an error is a typed value, and a function declares the errors it can fail with in its signature, using `throws`:

```csharp
int LoadScore() => throw new IOException();

try { var score = LoadScore(); }
catch (IOException e) { }
```

```fa
type IoError = Errors {
	NotFound
	Unreadable = { message: String }
}

function loadScore = (): Integer throws IoError => {
	throw IoError.NotFound
}

let score = try loadScore()                -- propagate to the caller
let score = loadScore() catch error => 0   -- handle it here
```

`try` is not a block: it marks one call whose errors propagate to the caller, which must declare them too. `catch` handles the errors of one expression and gives a replacement value. Every call that can fail is marked, so an error never travels silently through code that did not declare it, and there is no untyped `catch (Exception e)`. `??` only applies to optionals, never to errors. See [errors](/values/errors).

## Nullability that is not a retrofit

C#'s nullable reference types are a compiler analysis layered on a runtime where every reference can be null. In Fa, `none` inhabits only `T?`, at runtime as well as at compile time. There is no `!` null-forgiving operator, because there is nothing to forgive.

```fa
let user: User? = findUser(id)

console.log(user.name) -- error: `user` may be none

if user is not none {
	console.log(user.name) -- narrowed to `User`
}

let name = user?.name ?? "anonymous"
```

There is no truthiness either: `if user` does not compile, conditions only accept a `Boolean`.

## Pattern matching and unions

C#'s switch expressions are close, and Fa adds proper discriminated unions:

```fa
type Shape = Union {
	Circle = { radius: Float }
	Rect = { width: Float, height: Float }
}

let area = when shape is {
	Circle as { radius } => 3.14159 * radius ** 2
	Rect as { width, height } => width * height
}
```

`when` is exhaustive — no `default` needed, and adding a member breaks compilation until it is handled. There are no `when` guards: put an `if` inside the branch.

## LINQ becomes chained methods

```csharp
var total = numbers.Where(n => n % 2 == 0).Select(n => n * 2).Sum();
```

```fa
let total = numbers
	.filter((n) => n %% 2 == 0)
	.map((n) => n * 2)
	.reduce((sum, n) => sum + n, 0)
```

Chains are fused into one pass at compile time, so there is no iterator machinery and no allocation per stage — the same deferred-execution benefit LINQ gives, without the interface dispatch.

## Generics use parentheses

```csharp
T? First<T>(List<T> items)
```

```fa
function first(Item) = (items: Array(Item)): Item? => items.get(0)

first([1, 2, 3])
first(Float)([1, 2, 3])
```

Generic parameters come right after the name. They are inferred from the arguments and the expected type; when you need to write them, they are applied first: `first(Float)(...)`.

Constraints are structural rather than `where T : IComparable`:

```fa
function largest(Item: { value: Integer }) = (items: Array(Item)): Item? => ...
```

See [generics](/types/generics).

## Dependency injection becomes requirements

A function declares the services it needs with `requires`, and a caller provides them with `provide`. Nothing is passed by hand, and a missing service is a compile error, not a runtime exception:

```fa
type Logger = {
	info: (message: String) => Void
}

function saveUser = (user: User) requires logger: Logger => {
	logger.info("Saving {user.name}")
}

function main = () => {
	provide Logger = {
		info = (message) => console.log(message)
	}

	saveUser(currentUser)
}
```

See [requirements](/advanced/requirements).

## `async` without colour

An `async` function in Fa can also be called synchronously, so there is no `.Result`, no `GetAwaiter().GetResult()`, and no deadlock risk:

```fa
let data = try fetch(url)         -- called synchronously
let data = try await fetch(url)   -- awaited, inside an async function
let task = start fetch(url)       -- like Task.Run
```

Calling synchronously a primitive that is only asynchronous on a target (like network access on `web`) is a compile error. Several tasks are awaited with `Task.all`, `Task.any` or `Task.settled`.

There is no `ConfigureAwait`, and no `CancellationToken` to thread through — cancel the task itself:

```fa
let task = start slowOperation()
task.cancel()
```

## No garbage collector

Objects live on the stack, collections own the heap, and every value is released at the end of its scope. `IDisposable` and `using` become ordinary scoping:

```fa
do {
	mutable connection = pool.acquire()
	connection.send(message)
} -- released here
```

There is no `Dispose` to implement and no finaliser to get wrong.

## What you gain

- No `NullReferenceException`, and no null-forgiving escape hatch
- Typed errors and requirements in every signature, checked at compile time
- No GC pauses, no runtime, instant startup
- Compile-time execution instead of source generators and reflection
- Built-in reactivity — `INotifyPropertyChanged` becomes `reactive`
- Real exhaustiveness checking on unions

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `%` for remainder | `%%`, floor-based: `-7 %% 3 == 2`; `%` is the percentage suffix (`50%`) |
| `/` between integers truncates | `/` always returns a `Float`; `//` is floor division |
| `//` comments | `--`; `//` is floor division |
| Postfix `!` for null | does not exist; narrow with `is not none` |
| `with` expressions | spread: `{ ...user, age = 31 }`; `with` is partial application |
| `catch (Exception e)` | errors are declared with `throws`, and `catch` sees their exact types |
| `if (list.Any())` style truthiness | conditions only accept a `Boolean` |
| `class` vs `struct` | everything is a value; collections own the heap |
| `new User("Ada", 30)` | `User { name = "Ada", age = 30 }`; there are no positional constructors |
| Properties with getters/setters | plain fields, or methods |
| `ref` parameters | declare `mutates name` in the signature |
| `IDisposable`/`using` | scope-based release |
| Reflection | `@typeToSchema`, at compile time |
| `dynamic` | there is no dynamic typing |
