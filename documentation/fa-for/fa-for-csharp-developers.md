# Fa for C# developers

C# and Fa overlap more than most pairs: records, pattern matching, nullable reference types, `async`, LINQ-style chaining. Fa takes each of those further, and drops classes, exceptions and the garbage collector.

## Syntax at a glance

| C# | Fa |
| --- | --- |
| `// comment` | `-- comment` |
| `const`/`readonly var x = 1;` | `let x = 1` |
| `var x = 1;` | `mutable x = 1` |
| `int Add(int a) => a;` | `function add = (a: Int): Int => a` |
| `x => x + 1` | `(x: Int) => x + 1` |
| `$"Hello {name}"` | `"Hello {name}"` |
| `List<T>` | `Array(T)` |
| `Dictionary<K, V>` | `Map(K, V)` |
| `HashSet<T>` | `Set(T)` |
| `T?` | `T?` |
| `null` | `none` |
| `x?.y` | `x?.y` |
| `x ?? y` | `x ?? y` |
| `switch` expression | `when x is { }` |
| `foreach (var x in xs)` | `for xs >> x` |
| `x % y` | `x %% y` |
| `!x`, `&&`, `\|\|` | `not x`, `and`, `or` |
| `record Point(int X, int Y)` | `type Point = { x: Int, y: Int }` |
| `try` / `catch` / `throw` | error unions and `!` |
| `using var f = ...` | `use f = ...` |
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
  age = 0

  greet(self): String => "Hello, {self.name}"
}

let user = User { name = "Ada", age = 30 }
let user = User("Ada", 30)
let user = User(name = "Ada")
```

Every type in Fa behaves like a record: structural equality, structural printing, value semantics. There is no `class` versus `struct` decision, because everything is a value.

Non-destructive mutation, C#'s `with`, is the spread operator:

```fa
let older = { ...user, age = user.age + 1 }
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
function process = (Item: Type({ id: Int, name: String }), item: Item) => ...
```

Any type with those fields satisfies it, with nothing to declare. Fa is [structurally typed](../advanced/structural-typing) — this is C#'s implicit interface implementation, generalised to fields and applied everywhere.

## No exceptions

```csharp
int LoadScore() => throw new IOException();

try { var score = LoadScore(); }
catch (IOException e) { }
```

```fa
function loadScore = (): Int | IoError => ...

let score = loadScore()!
let score = loadScore() ?? 0

let score = when loadScore() is {
  Int >> value => value
  IoError >> error => { console.log(error.message); 0 }
}
```

The error type is in the signature, so it cannot be missed, and there is no stack unwinding. `!` propagates to the caller; it never throws.

## Nullability that is not a retrofit

C#'s nullable reference types are a compiler analysis layered on a runtime where every reference can be null. In Fa, `none` inhabits only `T?`, at runtime as well as at compile time. There is no `!` null-forgiving operator, because there is nothing to forgive.

```fa
let user: User? = findUser(id)

console.log(user.name) -- error

if user is not none {
  console.log(user.name)
}
```

:::caution
In C#, postfix `!` suppresses a null warning. In Fa, `!` propagates an **error** to the caller. They are unrelated.
:::

## Pattern matching and unions

C#'s switch expressions are close, and Fa adds proper discriminated unions:

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

`when` is exhaustive — no `default` needed, and adding a member breaks compilation until it is handled.

## LINQ becomes chained methods

```csharp
var total = numbers.Where(n => n % 2 == 0).Select(n => n * 2).Sum();
```

```fa
let total = numbers
  .filter((n: Int) => n %% 2 == 0)
  .map((n: Int) => n * 2)
  .reduce((acc: Int, n: Int) => acc + n, 0)
```

Chains are fused into one pass at compile time, so there is no iterator machinery and no allocation per stage — the same deferred-execution benefit LINQ gives, without the interface dispatch.

## Generics use parentheses

```csharp
T? First<T>(List<T> items)
```

```fa
function first = (Item: Type, items: Array(Item)): Item? => ...

first([1, 2, 3])
first(Item = Decimal, [1, 2, 3])
```

Constraints are structural rather than `where T : IComparable`:

```fa
function largest = (Item: Type({ value: Int }), items: Array(Item)): Item? => ...
```

## `async` without colour

An `async` function in Fa is callable from synchronous code, so there is no `.Result`, no `GetAwaiter().GetResult()`, and no deadlock risk:

```fa
let data = fetch(url)!         -- blocking
let data = await fetch(url)!   -- awaited
let task = start fetch(url)    -- like Task.Run
```

There is no `ConfigureAwait`, and no `CancellationToken` to thread through — cancel the task itself:

```fa
let task = start slowOperation()
task.cancel()
```

## No garbage collector

Objects live on the stack, collections own the heap, everything is released at the end of its scope. `IDisposable` and `using` become `use` and ordinary scoping:

```fa
do {
  use connection = pool.acquire()
  connection.send(message)
} -- released here
```

There is no `Dispose` to implement and no finaliser to get wrong.

## What you gain

- No `NullReferenceException`, and no null-forgiving escape hatch
- No GC pauses, no runtime, instant startup
- Compile-time execution instead of source generators and reflection
- Built-in reactivity — `INotifyPropertyChanged` becomes `reactive`
- Real exhaustiveness checking on unions

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `%` for remainder | `%%`; `%` builds a percentage |
| `//` comments | `--`; `//` is integer division |
| Postfix `!` for null | `!` propagates an error |
| `class` vs `struct` | everything is a value; collections own the heap |
| Properties with getters/setters | plain fields, or methods |
| `IDisposable`/`using` | `use` and scope-based release |
| Reflection | `@typeToSchema`, at compile time |
| `dynamic` | there is no dynamic typing |
