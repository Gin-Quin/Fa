# Parser AST coverage plan

Generic guidelines:

- For each new feature, add tests mirroring the examples in `documentation/`.
- Keep `parse_expression.rs` small by extracting feature-specific helpers into modules.

## Stage 1: AST generation
- [x] Add literal nodes: strings, symbols, none.
- [x] Implement operator nodes.
- [x] Implement functions.
- [x] Implement objects.
- [x] Implement namespaces.
- [x] Implement types.
- [x] Add union/intersection/difference type-expression nodes plus `Union {}`, `Fields {}`, `Enumeration` declarations and variant nodes.
- [x] Implement `break`, `mutable`, `static`, `continue` keywords.
- [x] Implement loops (for, while, loop).
- [x] Implement conditionals (if, else, else if)
- [x] Add member access (dot, bracket, string-path), optional chaining, `!`/`?` operators
- [x] Add percentages
- [x] Add error handling operators `!` and `?`
- [x] Add relation operator `->`
- [x] Add spread operator `...`
- [ ] Fix in match case `Result.Ok => ...`: using member access here seems to raise an error
- [x] Replace the `modulo` keyword with the `%%` operator
  - `tokenize.rs`: match `%%` before the single `%`, emitting a new `TokenKind::DoubleModulo`. Keep `b'%' => TokenKind::Percent` as the fallthrough, so the postfix percentage suffix is unaffected.
  - `tokenize.rs`: drop the `b"modulo"` keyword arm; `tokens.rs`: drop `TokenKind::Modulo`.
  - `parse_expression_right.rs:122`: retarget `List!(Modulo, operands, Priority::Multiplicative)` from `TokenKind::Modulo` to `TokenKind::DoubleModulo`. The node (`Node::Modulo`) and its priority stay as they are.
  - `typed_syntax_tree.rs:388`: render `Node::Modulo` as `" %% "` instead of `" modulo "`. Every existing test asserting `a modulo b` has to be updated.
  - Lexing edge case to cover: `%` is a postfix operator and `%%` is infix, so `a%%b` must lex as `a`, `%%`, `b` — never as `a%`, `%b`. Require a space to write a percentage followed by a remainder (`a% %% b`), and add a test pinning `a%%%b` to `(a %% (%b))` or to a lex error, whichever the parser ends up doing.
  - Tests: `tests/tokenize.rs` (`%%`, `%`, `%%=`), plus expression tests mirroring `documentation/operations/arithmetic.md`.
- [x] Add compound assignment operators, including `%%=`
  - None of these exist yet: there is no `PlusEqual`-style token, and `Priority::Assignment` only covers plain `=`. This is the whole family, not just `%%=`.
  - `tokens.rs` / `tokenize.rs`: add `+=`, `-=`, `*=`, `/=`, `//=`, `%%=`, `**=`. Each must be matched before its shorter prefix, and `//=` before `//` before `/`.
  - `parse_expression_right.rs`: parse them at `Priority::Assignment`, right-associative, into a single `Node::CompoundAssignment { operator, target, value }` rather than seven node kinds.
  - `typed_syntax_tree.rs`: render as `target <op>= value`.
  - Note for Stage 3: `//=` and `%%=` inherit the divisor checks, and every compound assignment inherits the range check on the destination type (see `documentation/operations/arithmetic.md`), so the type checker must treat `a += b` as `a = a + b` for range purposes.
  - Tests: `tests/assignment.rs`, mirroring the compound-assignment examples in `documentation/operations/arithmetic.md`.

## Stage 2: Handling symbols and types
- [ ] Read symbols and understand their scope.
- [ ] Implement a type resolver, that adds type information to an AST node (not all AST nodes have a type information).
- [ ] Handle hoisting functions.
- [ ] Read the file system to get an overview of a codebase.
- [ ] Add type-function nodes (lambda + invocation)
- [ ] Add built-in type ops (`Keys`, `Values`, `Pick`, `Omit`, etc.).

## Stage 3: Validation
- [ ] Implement a type checker, that validates the AST and the types.

## Stage 4: Generate Javascript
- [ ] Implement Rust code generation.

## Stage 5: Compile-time programming
- [ ] Support properties (get/set).
- [ ] Introduce object literals and constructor-style instantiation; add method/function-pointer declarations, self/mutable self handling.
- [ ] Model spread/extract (`...Parent`, `>> {}`) for values/types.

## Stage 6: Generate Rust
- [ ] Implement Rust code generation.

## Stage 7: Reactivity
- [ ] Handle Javascript and Rust code with a signal-based library

## Stage 8: Implement the standard library

On the JS side, the Bun api must be reproduced. We should first write a series of test to validate the Bun apis.

Then, the same api should be written in Rust.

- [ ] Write test files for the Bun api
- [ ] Implement the Bun api for Fa
- [ ] Write test files for the Rust api
