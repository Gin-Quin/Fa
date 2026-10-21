# Fa for Python developers

Fa is statically typed and compiled, but much of Python's readability carries over: no semicolons, no parentheses around conditions, keyword operators (`and`, `or`, `not`, `is`, `in`), keyword arguments, and string interpolation.

Fa is designed for AI agents as much as for humans: code reads almost naturally, everything a function does — the errors it throws, the values it requires, the data it mutates — is written in its signature, and if the code compiles, it works.

## Syntax at a glance

| Python | Fa |
| --- | --- |
| `# comment` | `-- comment` |
| `x = 1` | `let x = 1` (immutable) / `mutable x = 1` |
| `def add(a: int) -> int:` | `function add = (a: Integer): Integer => ...` |
| `lambda x: x + 1` | `(x) => x + 1` |
| `f"Hello {name}"` | `"Hello {name}"` |
| `int`, `float` | `Integer`, `Float` |
| `float(count)` | `Float(count)` |
| `list[str]` | `Array(String)` |
| `dict[str, int]` | `Map(String, Integer)` |
| `set[int]` | `Set(Integer)` |
| `tuple[int, str]` | `Tuple(Integer, String)` |
| `None` | `none` |
| `Optional[T]` | `T?` |
| `x is None` | `x is none` |
| `and`, `or`, `not`, `in` | `and`, `or`, `not`, `is in` |
| `x / y`, `x // y`, `x % y` | `x / y`, `x // y`, `x %% y` |
| `x ** y` | `x ** y` |
| `match x:` | `when x is { }` |
| `for x in xs:` | `for x in xs` |
| `for i, x in enumerate(xs):` | `for x, i in xs` |
| `for k, v in d.items():` | `for (k, v) in d` |
| `for i in range(10):` | `for i in 0..10` |
| `while (x := stack.pop()) is not None:` | `while let x = stack.pop()` |
| `xs[1:4]`, `xs[-1]` | `xs[1..4]`, `xs[-1]` |
| `raise NotFound()` | `throw LoadError.NotFound` |
| `try:` / `except E as e:` | `try call()` to propagate, `call() catch error => ...` to handle |
| `assert x == y` | `assert x == y` |
| `from m import f` | nothing — or `use { f } = m` |

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

Chained comparisons work as they do in Python, and each operand is evaluated once:

```fa
if 0 < score <= 100 {
	console.log("valid")
}
```

There is no truthiness, though: a condition must be a `Boolean`. `if items:` becomes `if items.length > 0`, and `if name:` becomes `if name is not none`.

## Everything is typed

Type annotations are checked, not hints, but inference means you rarely write them:

```fa
let name = "Ada"        -- String
let count = 42          -- Integer
let ratio = 0.5         -- Float
let names = ["Ada"]     -- Array(String)
```

`Integer` and `Float` are separate types, with no implicit conversion between them: write `Float(count) * ratio`. As in Python 3, `/` always returns a `Float` and `//` is floor division. Integers can also carry a range that the compiler checks:

```fa
let percent: Integer(min = 0, max = 100) = 50
```

## Errors are declared

Fa has `throw`, `try` and `catch`, but errors are typed values, not exceptions: a function declares the errors it can fail with in its signature, using `throws`, and the caller cannot forget them.

```python
def load_score() -> int:
    raise ValueError("boom")
```

```fa
type ScoreError = Errors {
	Missing
	Malformed = { line: Integer }
}

function loadScore = (): Integer throws ScoreError => {
	throw ScoreError.Missing
}

let score = try loadScore()                -- propagate to the caller
let score = loadScore() catch error => 0   -- or supply a fallback
```

`try` is not a block: it marks one call whose errors propagate to the caller, which must declare them too. To handle each error differently, match on it:

```fa
function printScore = () => {
	let score = loadScore() catch error => when error is {
		Missing => 0
		Malformed as { line } => {
			console.log("Bad line {line}")
			return
		}
	}
	console.log("Your score is {score}")
}
```

See [errors](/values/errors).

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
-- value: Integer | String
let label = when value is {
	0 => "zero"
	1..60 => "low"
	Integer as number => if number > 1000 { "huge" } else { "medium" }
	String => "text"
}
```

There are no guards (`case n if n > 1000`): put an `if` inside the branch. Destructuring uses `as`:

```fa
let area = when shape is {
	Circle as { radius } => 3.14159 * radius ** 2
	Rect as { width, height } => width * height
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
	age: Integer = 0
}

let user = User { name = "Ada" }
let user = User { name = "Ada", age = 30 }
```

There are no positional constructors: fields are always named. Equality is structural by default, so `==` compares field by field with nothing to implement.

Methods live in the type, and a method that changes its object declares it with `mutates self`:

```fa
type User = {
	name: String
	age: Integer = 0

	function greet = (self): String => "Hello, {self.name}"
	function birthday = (self) mutates self => {
		self.age += 1
	}
}
```

`self` is never annotated. There is no inheritance — composition uses `...`:

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
numbers.push(4) -- error: `numbers` is not mutable

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

Use `use` when you *want* an alias:

```fa
use b = a
```

A function that modifies a parameter says so in its signature:

```fa
function addBonus = (scores: Array(Integer), bonus: Integer) mutates scores => {
	for score in scores {
		score += bonus
	}
}
```

## Comprehensions become chained methods

```python
total = sum(x * 2 for x in numbers if x % 2 == 0)
```

```fa
let total = numbers
	.filter((x) => x %% 2 == 0)
	.map((x) => x * 2)
	.reduce((sum, x) => sum + x, 0)
```

The compiler fuses the chain into a single pass, so it is as fast as a hand-written loop.

## No imports

Files in a package see their siblings and the package root automatically. Each file exports exactly one thing, named after the file. To shorten a long path, create aliases with `use { formatDate } = utils`. See [modules](/architecture/modules).

## What you gain

- Errors and `none` that cannot be forgotten
- Real speed: native or WebAssembly, with no interpreter and no GC
- Compile-time execution — Python's decorators and code generation, but type-checked
- Typed requirements instead of global state, checked at compile time
- Built-in reactivity
- Genuine parallelism, with no GIL

## What to watch out for

| Habit | In Fa |
| --- | --- |
| `#` comments | `--`; `#` marks a private member |
| Significant indentation | braces; indentation is style only |
| `x % y` | `x %% y` (floor-based, like Python's `%`); `%` is the percentage suffix (`50%`) |
| `if items:` truthiness | conditions only accept a `Boolean` |
| Assigning a new name with `x = 1` | declare it with `let` or `mutable` first |
| Duck typing at runtime | structural typing at compile time |
| `d[key]` raising `KeyError` | `throws KeyNotFound` unless the key is proven; `.get(key)` returns `V?` |
| `xs[i]` raising `IndexError` | `throws IndexError` unless provably in bounds; `.get(i)` returns `T?` |
| `try` / `except` around a block | `try` or `catch` on each call that can fail, errors declared with `throws` |
| `from m import f as g` | `use { f as g } = m`; `as` only renames, it never converts |
| Mutable default arguments | defaults are values, evaluated once, and copied |
| `self` as the first parameter | `self`, but only in methods declared inside a type, and never annotated |
