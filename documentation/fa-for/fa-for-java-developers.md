# Fa for Java developers

Fa is statically typed and compiled, like Java, but it has no classes, no inheritance, no exceptions and no garbage collector. Most Java patterns have a shorter equivalent.

## Syntax at a glance

| Java | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `final int x = 1;` | `let x = 1` |
| `int x = 1;` | `mutable x = 1` |
| `int add(int a) { return a; }` | `function add = (a: Int): Int => a` |
| `(x) -> x + 1` | `(x: Int) => x + 1` |
| `List<T>` | `Array(T)` |
| `Map<K, V>` | `Map(K, V)` |
| `Set<T>` | `Set(T)` |
| `Optional<T>` | `T?` |
| `null` | `none` |
| `throws IOException` | `T \| IoError` in the return type |
| `try` / `catch` | error unions and `!` |
| `switch` | `when` |
| `for (T x : xs)` | `for xs >> x` |
| `x % y` | `x %% y` |
| `!x`, `&&`, `\|\|` | `not x`, `and`, `or` |
| `record Point(int x, int y)` | `type Point = { x: Int, y: Int }` |
| `sealed interface` + `record` | `type S = Union { ... }` |
| `interface Reader { ... }` | `Type({ read: () => Bytes })` |
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
  age = 0

  greet(self): String => "Hello, {self.name}"
}

let user = User { name = "Ada", age = 30 }
let user = User("Ada", 30)
let user = User(name = "Ada")
```

Equality, hashing and printing are structural by default: there is no `equals`, `hashCode` or `toString` to write.

Reuse is composition, not extension:

```fa
type Admin = {
  ...User
  permissions: Array(String)
}
```

An `Admin` is usable anywhere a `User` is expected, because Fa is [structurally typed](../advanced/structural-typing) — no `implements` clause required.

## No interfaces

An interface in Fa is a shape:

```fa
function process = (Item: Type({ id: Int, name: String }), item: Item) => ...
```

Every type with an `id` and a `name` satisfies it, without declaring anything. This removes the adapter classes that exist only to bridge two libraries' interfaces.

## No exceptions

Checked exceptions had the right goal; Fa achieves it with the return type:

```java
int loadScore() throws IOException { ... }

try {
	int score = loadScore();
} catch (IOException e) {
	// ...
}
```

```fa
function loadScore = (): Int | IoError => ...

let score = loadScore()!         -- propagate to the caller
let score = loadScore() ?? 0     -- default

let score = when loadScore() is {
  Int >> value => value
  IoError >> error => {
    console.log(error.message)
    0
  }
}
```

The error type is part of the signature, cannot be swallowed by an empty `catch`, and there is no unchecked/checked distinction. There is no stack unwinding at all.

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
  Circle = { radius: Decimal }
  Rect = { width: Decimal, height: Decimal }
}

let area = when shape is {
  Circle >> { radius } => 3.14159 * radius ** 2
  Rect >> { width, height } => width * height
}
```

`when` is exhaustive, so adding a member breaks compilation everywhere it needs handling.

## Generics use parentheses, and are real

```java
<T> Optional<T> first(List<T> items)
```

```fa
function first = (Item: Type, items: Array(Item)): Item? => ...

first([1, 2, 3])
first(Item = Decimal, [1, 2, 3])
```

There is **no erasure**: the type argument is available at compile time and drives specialisation. There is also no boxing — `Array(Int)` holds integers, not `Integer` objects.

## No garbage collector

Objects live on the stack and collections own the heap, all released at the end of their scope. No GC, no pauses, no tuning flags.

The cost is that assignment copies rather than sharing a reference:

```fa
mutable a = Point { x = 1 }
mutable b = a
b.x = 2
console.log(a.x) -- 1
```

Parameters and loop bindings are already references, so Java code translates without extra copying. Use `use` when you want an explicit alias. See [memory management](../advanced/memory-management).

## Concurrency

```fa
let results = await [
  start loadProfile(id)
  start loadOrders(id)
]

for parallel items >> item, index {
  results[index] = expensiveComputation(item)
}
```

An `async` function is callable from synchronous code, so there is no `CompletableFuture` plumbing and no reactive-streams library. Data races are impossible: values crossing a thread boundary are copied, and references cannot cross.

## Modules

There are no packages to declare and no imports to write. Files see their siblings and the package root; folders are namespaces; each file exports one thing. `#` is the private modifier. See [modules](../architecture/modules).

## What you gain

- No `NullPointerException`, no unchecked exceptions, no `ConcurrentModificationException`
- No GC pauses, no JVM, and a binary that starts instantly
- Compile-time execution instead of annotation processors and reflection
- Built-in reactivity
- Real generics, with no erasure and no boxing

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `%` for remainder | `%%`; `%` builds a percentage |
| `//` comments | `--`; `//` is integer division |
| `null` checks | `is none`, `?.`, `??` |
| `equals` / `hashCode` | structural, automatic |
| Builders | named arguments and default field values |
| Dependency injection | [contexts](../advanced/contexts), checked at compile time |
| `List<Integer>` boxing | `Array(Int)` stores integers directly |
| `int` overflow wrapping silently | rejected at compile time, or explicit with `wrappingAdd` |
