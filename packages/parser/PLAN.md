# Parser AST coverage plan

Generic guidelines:

- For each new feature, add tests mirroring the examples in `documentation/`.
- Keep `parse_expression.rs` small by extracting feature-specific helpers into modules.

## Stage 1: AST generation
- [x] Add literal nodes: strings, symbols, null.
- [x] Implement functions.
- [ ] Implement objects.
- [ ] Implement namespaces.
- [ ] Implement types.
- [ ] Implement loops (for, while, loop).
- [ ] Add member access (dot, bracket, string-path), optional chaining, `!`/`?` operators.
- [ ] Add extract and insert operators (`>>` and `<<`)
- [ ] Add union/intersection/difference type-expression nodes plus `Union {}`, `Fields {}`, `Enumeration` declarations and variant nodes.

## Stage 2: Handling symbols and types
- [ ] Read symbols and understand their scope.
- [ ] Implement a type resolver, that adds type information to an AST node (not all AST nodes have a type information).
- [ ] Handle hoisting functions.
- [ ] Read the file system to get an overview of a codebase.
- [ ] Add type-function nodes (lambda + invocation)
- [ ] Add built-in type ops (`Keys`, `Values`, `Pick`, `Omit`, etc.).

## Stage 3: Validation
- [ ] Implement a type checker, that validates the AST and the types.

## Stage 3: Generate Javascript
- [ ] Implement Rust code generation.

## Stage 4: Compile-time programming
- [ ] Support properties (get/set).
- [ ] Introduce object literals and constructor-style instantiation; add method/function-pointer declarations, self/mutable self handling.
- [ ] Model spread/extract (`...Parent`, `>> {}`) for values/types.

## Stage 5: Generate Rust
- [ ] Implement Rust code generation.

## Stage 6: Reactivity
- [ ] Add the `reactive` and `derived` keyword
- [ ] Handle Javascript and Rust code with a signal-based library

## Stage 6: Implement the standard library

On the JS side, the Bun api must be reproduced. We should first write a series of test to validate the Bun apis.

Then, the same api should be written in Rust.

- [ ] Write test files for the Bun api
- [ ] Implement the Bun api for Fa
- [ ] Write test files for the Rust api
