# Parser AST coverage plan

Generic guidelines:

- For each new feature, add tests mirroring the examples in `documentation/`.
- Keep `parse_expression.rs` small by extracting feature-specific helpers into modules.

## Stage 1: AST generation
- [x] Add literal nodes: strings, symbols, null.
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
- [ ] Add relation operator `->`

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
