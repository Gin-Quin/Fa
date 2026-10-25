# Fa

Fa is a programming language designed for **both AI agents and humans**.

Agents get a language they can't get wrong without the compiler noticing: strict types, typed errors, requirements and mutations, clear diagnostics and an unambiguous grammar.
Humans get code that reads almost like natural language, and that runs everywhere: web, server, native, WebAssembly.

Syntax-wise, Fa sits between Rust and TypeScript.

```fa
-- Load the score of the current user.
function loadScore = (): Integer throws ApiError requires session: Session => {
	let response = try fetchScore(session.user.id)
	response.score
}
```

## Status

Fa is in active development. Expect breaking changes.

## Built for AI agents

### 100% type safety

Like Rust, Fa has no escape hatch: no `any`, no unchecked casts, no `null` surprises. **If it compiles, it works.**
An agent can rely on the compiler as its feedback loop instead of on runtime testing.

### Effects in the syntax, "à la Effect"

Fa brings the ideas of the [Effect](https://effect.website) TypeScript library directly into the language, with no wrappers, generators or pipes to learn.
Every function signature tells you everything it can do:

- **Typed errors** (`throws`): there are no exceptions. Errors are values declared in the signature, and the compiler makes sure each one is either propagated with `try` or handled with `catch`.
- **Typed requirements** (`requires`): dependencies are passed implicitly through the call stack, but they are part of the signature. Calling a function without providing what it requires is a compile error.
- **Typed mutations** (`mutates`, `invalidates`): a function declares what it modifies, down to a field or an element, and which references it can invalidate.

```fa
type ApiError = Errors {
	NetworkError = { message: String }
	TimeoutError = { duration: Integer }
}

type Session = {
	user: { id: String, name: String }
}

function greet = () requires session: Session => {
	console.log("Hello, {session.user.name}!")
}

function main = () throws ApiError => {
	provide Session = { user = { id = "42", name = "Ada" } }

	greet() -- the session is provided implicitly

	-- `try` propagates the error if the call fails
	let score = try loadScore()

	-- `catch` handles it
	let scoreOrZero = loadScore() catch error => 0
}
```

### Clear error messages

Diagnostics explain what went wrong, where, and how to fix it. They are written to be acted upon by a human or an agent without guesswork.

### Simple, unambiguous syntax

Every construct has a single meaning and, as much as possible, a single way to be written. There are few keywords, no context-dependent parsing and no hidden rules, which makes Fa easy to read and easy to generate.

### Documentation in code

Documentation is part of the language, not a convention on top of it. A comment written right above a declaration is its documentation: it is attached to the declaration and are available to the compiler, the language server and the tooling, so agents always find the documentation right next to the code they work on.

## Built for humans

### Code that reads naturally

Fa favors words and simple operators over symbols. Code should read almost like a sentence:

```fa
for person in persons {
	if person.age is 18 {
		console.log("Welcome to adulthood, {person.name}!")
	}
}
```

### One language, many platforms

Fa is a **universal language**: write your code once, then compile it to **JavaScript** (browser, Bun, Node.js, Deno), **native**, **WebAssembly** or **Rust** source code.

The same code base can power your tooling, your applications and your games.

## A new take on memory management

Fa has no garbage collector and no borrow checker, yet it is 100% memory-safe. It relies on two concepts.

### Where data lives is part of the type

When you reference an element of a collection, its type says where it is stored. The compiler tracks the operations that can move or remove elements, as precisely as it can: after one of them, the references it may have broken must be declared again before being used.

```fa
mutable persons: Array(Person) = [{ name = "Ana" }, { name = "Bao" }, { name = "Cleo" }]

use someone = persons[1] -- type: `Person in persons`
someone.name = "John" -- fine: modifying an element keeps the references valid

persons.pop() -- fine: `pop` only invalidates the last element

persons.push({ name = "Dan" }) -- `push` may reallocate the array: every reference into `persons` is invalidated

console.log(someone.name) -- error: `someone` was invalidated by `persons.push`: declare it again with `use someone = persons[...]`
```

### Safe, explicit relations instead of weak pointers

A field that references another object names the collection it lives in, and is one side of a **relation** whose other side is declared too. The compiler keeps both sides in sync, and deleting an object cleans up every relation it was part of, so no reference is ever left dangling.

```fa
mutable people: Bag(Person) = []

type Person = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: Bag(people->bestFriend) -- the people whose `bestFriend` is me
}

use ana = people.add({ name = "Ana" })
use bao = people.add({ name = "Bao" })

ana.bestFriend = bao -- `bao.bestFriendOf` is updated automatically
people.delete(bao) -- `ana.bestFriend` becomes `none`
```

## Documentation

The full documentation lives in [`documentation/`](documentation) and is published by the website in [`packages/website`](packages/website).

## Getting started

### Prerequisites

To test Fa locally, you need Rust installed. Install it with `rustup` from https://rustup.rs.

### Build the parser

The parser project lives in `packages/parser` and builds as the `fa-parser` crate.

From the repository root:

```bash
cargo build -p fa-parser
```

### Run the parser

The parser binary expects a file path and prints the parsed tree.

```bash
cargo run -p fa-parser -- path/to/file.fa
```

### Hello world

```fa
console.log("Hello, world")
```
