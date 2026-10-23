# Fa for Java developers

Fa is statically typed and compiled, like Java, but it has no classes, no inheritance, no exceptions and no garbage collector. Most Java patterns have a shorter equivalent.

Fa is designed for AI agents as much as for humans: everything a function does — the errors it throws, the services it requires, the data it mutates — is written in its signature, and if the code compiles, it works.

## Syntax at a glance

| Java | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `final int x = 1;` | `let x = 1` |
| `int x = 1;` | `mutable x = 1` |
| `int add(int a) { return a; }` | `function add = (a: Integer): Integer => a` |
| `(x) -> x + 1` | `(x) => x + 1` |
| `long`, `double` | `Integer`, `Float` |
| `(double) count` | `Float(count)` |
| `List<T>` | `Array(T)` |
| `Map<K, V>` | `Map(K, V)` |
| `Set<T>` | `Set(T)` |
| `Optional<T>` | `T?` |
| `null` | `none` |
| `int load() throws IOException` | `function load = (): Integer throws IoError => ...` |
| `throw new NotFoundException()` | `throw IoError.NotFound` |
| `try { ... } catch (E e) { ... }` | `try call()` to propagate, `call() catch error => ...` to handle |
| `switch` | `when` |
| `for (T x : xs)` | `for x in xs` |
| `x % y` | `x %% y` |
| `!x`, `&&`, `\|\|` | `not x`, `and`, `or` |
| `record Point(int x, int y)` | `type Point = { x: Integer, y: Integer }` |
| `new Point(1, 2)` | `Point { x = 1, y = 2 }` |
| `sealed interface` + `record` | `type S = Union { ... }` |
| `interface Reader { ... }` | a structural constraint: `{ read: () => Array(Byte) }` |
| `assert x == y;` | `assert x == y` |
| `import java.util.List;` | nothing — files see their siblings |

## No classes

There are objects and types, but no classes, no constructors to write, and no inheritance.

```java
public record User(String name, int age) {
	public String greet() {
		return "Hello, " + name;
	}
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

Equality, hashing and printing are structural by default: there is no `equals`, `hashCode` or `toString` to write, and `==` compares values deeply.

A method that changes its object says so with `mutates self`, and can only be called on a `mutable` value:

```fa
type Account = {
	balance: Integer = 0

	function deposit = (self, amount: Integer(min = 0)) mutates self => {
		self.balance += amount
	}
}
```

Reuse is composition, not extension:

```fa
type Admin = {
	...User
	permissions: Array(String)
}
```

An `Admin` is usable anywhere a `User` is expected, because Fa is [structurally typed](/advanced/structural-typing) — no `implements` clause required.

## No interfaces

An interface in Fa is a shape:

```fa
function process(Item: { id: Integer, name: String }) = (item: Item) => ...
```

Every type with an `id` and a `name` satisfies it, without declaring anything. This removes the adapter classes that exist only to bridge two libraries' interfaces.

## Errors are values, not exceptions

Checked exceptions had the right goal, and Fa keeps the `throws` clause. But an error is a typed value returned to the caller, not an exception unwinding the stack:

```java
int loadScore() throws IOException { ... }

try {
	int score = loadScore();
} catch (IOException e) {
	// ...
}
```

```fa
type IoError = Errors {
	NotFound
	Unreadable = { message: String }
}

function loadScore = (): Integer throws IoError => ...

let score = try loadScore()                -- propagate to the caller
let score = loadScore() catch error => 0   -- handle it here
```

`try` is not a block: it marks one call whose errors propagate, and the enclosing function must declare them in its own `throws`. `catch` handles the errors of one expression and gives a replacement value. To handle each error differently, match on it — the compiler checks that every member is handled:

```fa
function printScore = () => {
	let score = loadScore() catch error => when error is {
		NotFound => 0
		Unreadable as { message } => {
			console.log("Cannot read the score: {message}")
			return
		}
	}
	console.log("Your score is {score}")
}
```

There are no unchecked exceptions: every error is in a signature, and every call that can fail is marked with `try` or `catch`. See [errors](/values/errors).

## No null

`none` inhabits only types written with `?`, and the compiler makes you handle it:

```fa
let user: User? = findUser(id)

console.log(user.name) -- error: `user` may be none

if user is not none {
	console.log(user.name)
}

let name = user?.name ?? "anonymous"
```

`Optional<T>` and its combinators are unnecessary: `?.` and `??` are built in.

## Sealed hierarchies become unions

```java
sealed interface Shape permits Circle, Rect {}
record Circle(double radius) implements Shape {}
record Rect(double w, double h) implements Shape {}
```

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

`when` is exhaustive, so adding a member breaks compilation everywhere it needs handling.

## Generics use parentheses, and are real

```java
<T> Optional<T> first(List<T> items)
```

```fa
function first(Item) = (items: Array(Item)): Item? => items.get(0)

first([1, 2, 3])
first(Float)([1, 2, 3])
```

Generic parameters come right after the name, and are inferred from the arguments and the expected type. There is **no erasure**: the type argument is available at compile time and drives specialisation. There is also no boxing — `Array(Integer)` holds 64-bit integers directly, not objects. See [generics](/types/generics).

## Dependency injection becomes requirements

A function declares the services it needs with `requires`, and a caller provides them with `provide`. There is no container, no annotation and no reflection, and a missing service is a compile error:

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

## No garbage collector

Objects live on the stack and collections own the heap, all released at the end of their scope. No GC, no pauses, no tuning flags.

The cost is that assignment copies rather than sharing a reference:

```fa
mutable a = Point { x = 1, y = 2 }
mutable b = a
b.x = 3
console.log(a.x) -- 1
```

Parameters and loop bindings are already references, so Java code translates without extra copying. Use `use` when you want an explicit alias. See [memory management](/advanced/memory-management).

## Concurrency

```fa
let pages = await Task.all([start loadProfile(id), start loadOrders(id)])

for parallel item, index in items {
	results[index] = expensiveComputation(item)
}
```

An `async` function can also be called synchronously, so there is no `CompletableFuture` plumbing and no reactive-streams library. Data races are impossible: values crossing a thread boundary are copied, references cannot cross, and only thread-safe types (mutexes, atomics, channels) can be shared mutably. See [concurrency](/advanced/concurrency).

## Modules

There are no packages to declare and no imports to write. Files see their siblings and the package root; folders are namespaces; each file exports one thing. `#` is the private modifier. See [modules](/architecture/modules).

## What you gain

- No `NullPointerException`, no unchecked exceptions, no `ConcurrentModificationException`
- Typed errors and requirements in every signature, checked at compile time
- No GC pauses, no JVM, and a binary that starts instantly
- Compile-time execution instead of annotation processors and reflection
- Built-in reactivity
- Real generics, with no erasure and no boxing

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `%` for remainder | `%%`, floor-based: `-7 %% 3 == 2`; `%` is the percentage suffix (`50%`) |
| `/` between integers truncates | `/` always returns a `Float`; `//` is floor division |
| `//` comments | `--`; `//` is floor division |
| `Integer` as a boxed object | `Integer` is a plain 64-bit integer |
| `null` checks | `is none`, `?.`, `??` |
| `equals` / `hashCode` | structural, automatic |
| `new User(...)` | `User { name = "Ada" }`; there are no constructors |
| Builders | named fields and default field values |
| Dependency injection | [requirements](/advanced/requirements), checked at compile time |
| `catch (Exception e)` | errors are declared with `throws`, and `catch` sees their exact types |
| Modifying the list while iterating | a compile error |
| `int` overflow wrapping silently | a compile error when certain, a runtime stop when possible, or explicit with `wrappingAdd` |
