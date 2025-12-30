# Repository Guidelines

## Project Structure & Module Organization
Fa is organized as a Rust workspace rooted in `Cargo.toml`, with active crates under `packages/` (`parser`, `codebase-parser`, `language-server-protocol`). Shared experimental code lives in `sandbox/`, and built artifacts land in `target/`. The developer-facing site is under `packages/documentation` (SvelteKit) and sources Markdown from the top-level `documentation/` directory. The documentation lives in `documentation/`, which automatically generates the routes in `packages/documentation/src/routes/` using the `scripts/createDocumentationRoutes.ts` script. The specs for the Fa language lives in `documentation/`. If you find any ambiguity, indicate it to me before writing or editing code and ask a question how to resolve the ambiguity. Keep language assets grouped by package; avoid duplicating source across crates.

## Build, Test, and Development Commands
- `just install`: installs Bun or Cargo dependencies for all packages.
- `cargo build --workspace`: compiles all Rust crates; use `-p <crate>` for granular builds.
- `cargo check --workspace`: fast type-check for Rust code before committing.
- `cargo test --workspace`: runs all Rust test suites.
- `cargo test -p <crate> <test_name>`: runs a specific test (e.g., `cargo test -p fa-parser function_calls_no_parameters`).
- `cargo test -- <test_name>`: runs all tests matching the pattern across workspace.
- `cargo fmt --all`: formats all Rust code to project standards (hard tabs).
- `cargo clippy --workspace --all-targets`: lints all Rust code and catches common mistakes.
- `bun run dev` (from `packages/documentation`): starts the docs site with live route generation.
- `bun run build` (same directory): produces a static documentation bundle.

## Coding Style & Naming Conventions

### Rust Code
- **Edition**: Use Rust 2024 edition.
- **Indentation**: Hard tabs (defined in `.rustfmt.toml`).
- **Files**: snake_case filenames (e.g., `parse_expression.rs`, `tokenize.rs`).
- **Types**: CamelCase for enums, structs, and type aliases (e.g., `Node`, `TokenKind`, `TypedSyntaxTree`).
- **Functions/Methods**: snake_case (e.g., `parse_expression`, `tokenize`, `parse_statement`).
- **Constants**: SCREAMING_SNAKE_CASE (e.g., `MAX_DEPTH`, `DEFAULT_PRIORITY`).
- **Modules**: snake_case matching file/directory names.
- **Macros**: Macro names follow CamelCase convention (e.g., `Prefix!`, `Operation!`).

### Imports
Group imports in this order:
1. Standard library (`std::*`)
2. External crates (lsp-types, tempfile, etc.)
3. Local modules (`crate::*`)

```rust
use std::fs::{self, File};
use std::path::Path;

use tempfile::TempDir;

use crate::context::Context;
use crate::nodes::Node;
```

### Error Handling
- Use `Result<T, E>` for recoverable errors.
- Use `expect()` or `unwrap()` only when failure is truly exceptional or impossible.
- Use `panic!()` for programmer errors (e.g., unreachable states, violated invariants).
- Prefer the `?` operator for propagating errors.

### Svelte & TypeScript
- **Components**: PascalCase filenames for Svelte component and files that export a single Type (e.g., `Home.svelte`, `FancyButton.svelte`, `SomeType.ts`).
- **Routes**: camelCase for other filenames or folders (e.g., `gettingStarted/introduction/hello.ts`).
- **TypeScript**: strict mode enabled, use explicit types where helpful.
- **Imports**: Imports are automatically sorted when formatted with Biome.

### Testing
- Unit tests: colocate in `src/` as modules with `#[cfg(test)]`.
- Integration tests: place in crate-level `tests/` directories.
- Test names: descriptive snake_case that mirrors behavior (`function_calls_no_parameters`, `parses_expression_with_operators`).
- Helper functions: prefix with `test_` or `assert_` to indicate test utilities.
- Gate long-running tests with `#[ignore]` (e.g., `#[test] #[ignore] fn integration_test()`).
- Use `setup_test_directory()` pattern for fixtures requiring temp directories.

## Parser Package Notes (`packages/parser`)

### How It Works
- The entry points are `parse` and `parse_single_statement` in `packages/parser/src/parse.rs`. Both call `tokenize` then drive `parse_statement`.
- `tokenize` (`packages/parser/src/tokenize.rs`) converts the input into `Token { kind, start, end }` and normalizes newlines into `TokenKind::Stop`, collapsing redundant stops.
- `parse_expression` (`packages/parser/src/parse_expression.rs`) parses a left expression, then repeatedly parses right expressions (operators, calls, assignments) based on `Priority`.
- `Node` in `packages/parser/src/nodes.rs` is the typed syntax tree node enum. `TypedSyntaxTree` stores nodes by index and `node_to_string` in `packages/parser/src/typed_syntax_tree.rs` renders them.

### Minimal Examples

```rust
use fa_parser::parse;

let tree = parse("a + b");
assert_eq!(tree.to_string(), "(a + b);");
```

```rust
use fa_parser::parse_single_statement;

let tree = parse_single_statement("function add = (a, b) => a + b");
assert_eq!(tree.to_string(), "function add = (a, b) => (a + b)");
```

### Adding a New Node / Syntax
- **Token (if new syntax):** add/adjust tokenization in `packages/parser/src/tokenize.rs` and `packages/parser/src/tokens.rs`.
- **Node:** add a variant in `packages/parser/src/nodes.rs`.
- **Parsing:** wire it in `packages/parser/src/parse_expression.rs` (prefix or infix) or create a helper module and call it from there.
- **Priority:** update `packages/parser/src/priority.rs` if the operator needs a new precedence.
- **Rendering:** implement formatting in `packages/parser/src/typed_syntax_tree.rs`.
- **Tests:** add or extend tests under `packages/parser/src/tests/` (see existing `expressions.rs`, `functions.rs` for patterns).

### Practical Advice
- If you add a prefix keyword (e.g. `foo <expr>`), follow the `Prefix!` pattern in `parse_expression` and assign a `Priority::PrefixKeyword`.
- If you add a binary operator, follow the `List!` or `Operation!` pattern in `parse_expression_right` so precedence and left-associativity work correctly.
- For new grouped constructs, add a helper like `parse_members` and keep newline skipping behavior consistent with existing group parsers.

## Before Committing
Always run these commands:
```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```
For parser changes, ensure existing tests pass and add new tests for new behaviors.

## Commit & Pull Request Guidelines
- Follow the existing convention: emoji prefix plus concise, present-tense summary.
  - Examples: `✨ Add tuple pattern support`, `🐛 Fix function parsing edge case`, `📝 Update installation docs`
- Keep subjects under ~72 characters.
- Include body bullets for context when needed.
- Link relevant issues (if any).
- Note manual test commands for visible/CLI changes.
- Include screenshots or terminal logs for UI changes.
- Small, well-scoped PRs review fastest; split parser and frontend updates when practical.
