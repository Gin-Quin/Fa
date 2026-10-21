# Fa for Python developers

Fa is statically typed and compiled, but much of Python's readability carries over: no semicolons, no braces around conditions, keyword operators (`and`, `or`, `not`, `is`, `in`), keyword arguments, and string interpolation.

## Syntax at a glance

| Python | Fa |
| --- | --- |
| `# comment` | `-- comment` |
| `x = 1` | `let x = 1` (immutable) / `mutable x = 1` |
| `def add(a: int) -> int:` | `function add = (a: Int): Int => ...` |
| `lambda x: x + 1` | `(x: Int) => x + 1` |
| `f"Hello {name}"` | `"Hello {name}"` |
| `list[str]` | `Array(String)` |
| `dict[str, int]` | `Map(String, Int)` |
| `set[int]` | `Set(Int)` |
| `tuple[int, str]` | `Tuple(Int, String)` |
| `None` | `none` |
| `Optional[T]` | `T?` |
| `x is None` | `x is none` |
| `and`, `or`, `not`, `in` | `and`, `or`, `not`, `is in` |
| `x % y` | `x %% y` |
| `x // y` | `x // y` |
| `x ** y` | `x ** y` |
| `match x:` | `when x is { }` |
| `for x in xs:` | `for xs >> x` |
| `for i, x in enumerate(xs):` | `for xs >> x, i` |
| `for k, v in d.items():` | `for d >> (k, v)` |
| `range(10)` | `0..10` |
| `xs[1:4]`, `xs[-1]` | `xs[1..4]`, `xs[-1]` |
| `try` / `except` / `raise` | error unions and `!` |
| `from m import f` | nothing — or `m >> use { f }` |

Indentation is not significant in Fa: blocks use braces. Newlines separate statements, and there are no semicolons.

## Blocks and conditions

```python
if score >= 50:
    print("pass")
else:
    print("fail")
```

```fa
if score >= 50 {
  console.log("pass")
} else {
  console.log("fail")
}
```

Chained comparisons work as they do in Python:

```fa
if 0 < score <= 100 {
  console.log("valid")
}
```

## Everything is typed

Type annotations are checked, not hints, but inference means you rarely write them:

```fa
let name = "Ada"        -- String
let count = 42          -- Int
let ratio = 0.5         -- Decimal
let names = ["Ada"]     -- Array(String)
```

There is no single `int`/`float` pair: integers and decimals are separate, and integers can carry a range that the compiler checks:

```fa
let percent: Int(min = 0, max = 100) = 50
```

## No exceptions

`raise` and `except` do not exist. A function that can fail says so, and the caller cannot forget:

```python
def load_score() -> int:
    raise ValueError("boom")
```

```fa
type ScoreError = Errors {
  Missing
  Malformed = { line: Int }
}

function loadScore = (): Int | ScoreError => ScoreError.Missing

let score = loadScore()!         -- propagate to the caller
let score = loadScore() ?? 0     -- or supply a default

let score = when loadScore() is {
  Int >> value => value
  Missing => 0
  Malformed >> error => {
    console.log("bad line {error.line}")
    0
  }
}
```

## No `None` surprises

`none` only inhabits types that opted in with `?`, and you must handle it before use:

```fa
let user: User? = findUser(id)

console.log(user.name) -- error: `user` may be none

if user is not none {
  console.log(user.name)
}

let name = user?.name ?? "anonymous"
```

## Pattern matching

`when` is Python's `match`, and is exhaustive:

```fa
let label = when value is {
  0 => "zero"
  0..60 => "low"
  Int >> n if n > 1000 => "huge"
  String => "text"
  else => "other"
}
```

Destructuring uses `>>`:

```fa
when shape is {
  Circle >> { radius } => 3.14159 * radius ** 2
  Rect >> { width, height } => width * height
}
```

## Dataclasses are just types

```python
@dataclass
class User:
    name: str
    age: int = 0
```

```fa
type User = {
  name: String
  age = 0
}

let user = User { name = "Ada" }
let user = User("Ada", 30)
let user = User(name = "Ada", age = 30)
```

Equality is structural by default, so `==` compares field by field with nothing to implement.

Methods live in the type:

```fa
type User = {
  name: String
  age = 0

  greet(self): String => "Hello, {self.name}"
  birthday(mutable self) => { self.age += 1 }
}
```

There is no inheritance and no `self` parameter to type — composition uses `...`:

```fa
type Admin = {
  ...User
  permissions: Array(String)
}
```

## Mutability is explicit

Python's lists and dicts are always mutable. In Fa, `let` is deeply immutable and `mutable` opts in:

```fa
let numbers = [1, 2, 3]
numbers.push(4) -- error

mutable numbers = [1, 2, 3]
numbers.push(4) -- fine
```

And assignment copies rather than sharing a reference — no more surprise aliasing:

```python
a = [1, 2]
b = a
b.append(3)
print(a)  # [1, 2, 3]
```

```fa
mutable a = [1, 2]
mutable b = a
b.push(3)
console.log(a) -- [1, 2]
```

Use `use` when you *want* to share:

```fa
use b = a
```

## Comprehensions become chained methods

```python
total = sum(x * 2 for x in numbers if x % 2 == 0)
```

```fa
let total = numbers
  .filter((x: Int) => x %% 2 == 0)
  .map((x: Int) => x * 2)
  .reduce((acc: Int, x: Int) => acc + x, 0)
```

The compiler fuses the chain into a single pass, so it is as fast as a hand-written loop.

## No imports

Files in a package see their siblings and the package root automatically. Each file exports exactly one thing, named after the file. See [modules](../architecture/modules).

## What you gain

- Errors and `none` that cannot be forgotten
- Real speed: native or WebAssembly, with no interpreter and no GC
- Compile-time execution — Python's decorators and code generation, but type-checked
- Built-in reactivity
- Genuine parallelism, with no GIL

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `#` comments | `--`; `#` marks a private member |
| Significant indentation | braces; indentation is style only |
| `x % y` | `x %% y`; `%` builds a percentage |
| Duck typing at runtime | structural typing at compile time |
| `d[key]` raising `KeyError` | returns `V \| KeyNotFound`; `.get()` returns `V?` |
| `xs[i]` raising `IndexError` | returns `T \| IndexError` unless provably in bounds |
| `try` / `except` | error unions and `!` |
| Mutable default arguments | defaults are values, evaluated once, and copied |
| `self` as the first parameter | `self`, but only for methods, and never annotated |
