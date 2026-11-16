# Repository Guidelines

## Project Structure & Module Organization
Fa is organized as a Rust workspace rooted in `Cargo.toml`, with active crates under `packages/` (`parser`, `codebase-parser`, `language-server`, `analyzer`, `interpreter` (compile-time evaluation, lowered from the analysis), `codegen-llvm`, `codegen-js`, `formatter` (`fa format`), `cli` (the `fa` command); see `design/compiler-architecture.md` for the planned crates). Shared experimental code lives in `sandbox/`, and built artifacts land in `target/`. The website is under `packages/website` (SvelteKit, without SveltePress): a landing page, and the documentation rendered at build time from the Markdown files of the top-level `documentation/` directory. Every Markdown page must be listed in `packages/website/src/lib/docs/navigation.ts` (the build fails otherwise), and the Fa snippets of the landing page live in `packages/website/src/lib/landing/snippets.ts`. A `:::implementation` … `:::` section of a page holds the implementation details (layouts, algorithms, native and JavaScript compilation): the website removes it, and you must read it before implementing the feature it describes (see `documentation/README.md`). The specs for the Fa language lives in `documentation/`. Design records that are not language pages (the native compilation strategy, the spec review, the [journal](design/journal.md), decisions and the single open-work list `design/todo.md`) live in `design/`. If you find any ambiguity, indicate it to me before writing or editing code and ask a question how to resolve the ambiguity. Keep language assets grouped by package; avoid duplicating source across crates.

## Build, Test, and Development Commands
- `just install`: installs Bun or Cargo dependencies for all packages.
- `cargo build --workspace`: compiles all Rust crates; use `-p <crate>` for granular builds.
- `just setup-llvm` (or `scripts/setup-llvm.sh`): downloads LLVM 18 and lld with `apt-get download` and extracts them into `~/.local/llvm18` without root, then prints the variable to export: `export LLVM_SYS_181_PREFIX="$HOME/.local/llvm18/usr/lib/llvm-18"`. The `fa-codegen-llvm` crate (`packages/codegen-llvm`) compiles its LLVM code and runs its LLVM tests only when this variable is set at build time; otherwise its tests print a skip message, so `cargo test --workspace` works without LLVM. `cargo run -p fa-codegen-llvm --release --example report` prints binary sizes and timings. `cargo run -p fa-codegen-llvm --example build -- file.fa --target=native|wasm [--run -- arguments…]` compiles a standalone file; its tests run the programs of `packages/codegen-llvm/tests/programs/` and the printing cases of the analyzer on every target. The C part of its runtime (`packages/codegen-llvm/runtime/fa_runtime.c`) is checked in as IR: after editing it, run `packages/codegen-llvm/runtime/generate.sh` (needs `FA_LLVM_WITH_CLANG=1 scripts/setup-llvm.sh`).
- `cargo check --workspace`: fast type-check for Rust code before committing.
- `cargo test --workspace`: runs all Rust test suites.
- `cargo test -p <crate> <test_name>`: runs a specific test (e.g., `cargo test -p fa-parser function_calls_no_parameters`).
- `cargo test -- <test_name>`: runs all tests matching the pattern across workspace.
- `bun test packages/codegen-js/runtime`: runs the tests of the JavaScript runtime of Fa (`packages/codegen-js/runtime/*.js`, plain ES2022 modules with JSDoc). Each helper starts at a `// @fa-helper <name> deps: <helpers>` line, which `fa-codegen-js` uses to extract a helper with its dependencies (`collect_helpers`); its Rust tests check that the declared dependencies match the code and that every helper runs alone in Bun.
- `cargo run -p fa -- check|build|run|test <file.fa | package>` and `fa new package <name>`: the `fa` command (`packages/cli`, `src/project.rs` finds the program): a `.fa` file is a standalone program with the files next to it (a file of `scripts/` runs in its package), a folder (or none: the current one) is the closest package (`package.fa`, else `CLI001`). `check` prints the diagnostics of every file, `build --target=bun|node|web [--output=<dir>]` writes `main.js` (a standalone file using no other file) or one ES module per Fa file plus `main.js` and `$runtime.js` (default `build/<target>/`, `<package>/<build.output>/<target>/` for a package), `run` compiles for Bun into `~/.cache/fa/run/` and runs it with Bun (`PATH` or `~/.bun/bin`), forwarding the arguments after the file and the exit code; `test` runs the `*.test.fa` files' tests on Bun (`skip`/`only`, fresh module-level values per test, `TEST004`; `--watch` runs them again after each change); `new package` scaffolds the hello-world package (its name must be an identifier, `CLI008`), `new workspace [<name>]` a workspace. For `native`, `native-freestanding` and `wasm`, a package is compiled per unit of the pipeline and linked (ThinLTO for `fa build`, cached objects for `fa run`; `FA_SPLIT=0` compiles it as one module, `FA_LTO=full` links with whole-program LTO; step 6 of `design/parallel-pipeline.md`). `build` of a package without `--target` builds every target of its `targets` (each outcome reported); `--output=<dir>` writes into `<dir>/<target>/`; `check --format=json` prints one JSON object per diagnostic on stdout; usage errors exit with 1. `fa run --target=native` and `fa test --target=native` don't link an executable: LLVM's JIT loads the program in the process of `fa-native` (`packages/codegen-llvm/src/jit.rs`: the object, or the cached objects of a package, linked in memory; thread-local variables emulated), `fa run` runs it there and `fa test` compiles the tests once without optimization and forks one child per test; `FA_JIT=0` keeps the linked executable (as do a program that links C libraries and a program the JIT can't load), `FA_JIT_TRACE=1` says which one runs; tested by `cargo test -p fa --test jit`. The `fa` binary does not link LLVM (startup time): the `native`/`wasm` targets of `build` and `run` are compiled by the second binary of the package, `fa-native` (`src/native_main.rs`: the same command line with the LLVM backend registered, `src/native.rs`), which `fa` finds next to its own executable, else on the `PATH` (`cargo build -p fa` builds both). Every tool finds its project with `fa_codebase_parser::discover`. In a workspace (`workspace.fa`, `packages/` scanned by `fa_codebase_parser::scan_workspace`, `src/workspace.rs`), the commands take a package name, and without one run for every package that implements them. `develop [<package>]` runs the application on Bun and swaps the edited modules in the running program, or restarts it (`src/develop.rs`: polling watcher, `Session::update`; `src/hot.rs`: hot reloading by module swap, D43), `preview` runs the output of `fa build`, `update [<key>...]` re-resolves the dependencies, `help [<command>]` describes a command. Dependencies (`src/dependencies.rs`): sibling packages, Git sources through the `git` command, remote files and folders, `dependencies.lock` (SHA-256, D45) and the cache (`$FA_CACHE`), mounted for the analyzer by `fa_analyzer::mounts::MountedResolver`; see "Workspaces, dependencies and development mode" in `design/decisions.md`.
- `cargo test -p fa --test programs`: the end-to-end tests: every `packages/cli/tests/programs/*.fa`, runnable analyzer case with `-- prints:` / `-- exit:`, and LLVM `async-*.fa` program runs on Bun (`fa run`), Node (`fa build --target=node`, then Node), and a Node VM exposing web-standard globals (`fa build --target=web`). Missing Bun skips its tests; missing Node (`~/.local/bin/node`, then PATH) skips Node/web. All targets share stdout, exit-code, `-- stderr:` substring and `-- args:` comparisons, `-- overflow-checks:`, `-- asynchronous:` and `-- skip-target:` (including `bun`, `node`, `web`). Node/web count documented target refusals as negative tests (`AN004`, `FFI008`, `CONC001`, `CONC010`, and JavaScript integer limits `E-NUM-19`); other failures are ratcheted in `packages/cli/tests/expected-skips/{node,web}.txt`: unlisted failures, listed successes and stale entries fail. `FA_UPDATE_SKIPS=1` refreshes the lists only on a full run; review the diff. `FA_PROGRAMS=filter` selects paths, `FA_VERBOSE=1` lists results. Three programs run concurrently across all targets, with a 20-second timeout and a 512 MiB resident-memory limit per child on Linux. Bun retains its existing `-- pending:` behavior; Node/web run those programs against their ratchets. See the testing notes in `design/compiler-architecture.md`.
- `cargo test -p fa --test packages`: the multi-file end-to-end tests: the package fixtures of `packages/cli/tests/packages/` through `fa run`/`fa test`/`fa build`, `fa new package`, and every multi-file analyzer case (`-- file:` sections) with `-- prints:`.
- `cargo test -p fa --test workspaces`: the workspace fixture of `packages/cli/tests/workspaces/`, Git dependencies served by local bare repositories (`FA_GITHUB_BASE`, `git+file://`), remote files (`file://`), the lockfile and `fa update`, and `fa develop` / `fa test --watch` with a scripted change (`FA_DEVELOP_RUNS`); offline.
- `cargo test -p fa --test c_library` and `cargo test -p fa-codegen-llvm --test c_library`: a package built as a C library (`build.cLibrary`: `packages/cli/tests/c-library/`, `lower/c_library.rs`, `header.rs`): its header compiled with `-std=c11 -Wall -Wextra -Werror -pedantic`, its C program linked statically and dynamically and run (under Valgrind when installed), loaded with `dlopen`, the leak check at unload; the checks of the API (`FFI012`–`FFI014`, `REQ006`).
- `cargo test -p fa-codegen-js --test golden`: compares the JavaScript generated for `packages/codegen-js/tests/golden/*.fa` with the `.js` files next to them; `FA_UPDATE_GOLDEN=1` rewrites them (review the diff).
- `cargo run -p fa -- format [<files or folders>...] [--check]`: formats Fa code with `fa-formatter` (`packages/formatter`, choices in `design/decisions.md`); `cargo test -p fa-formatter --test corpus -- --nocapture` checks the tree preservation, the idempotence and the comments on every ```fa block of the documentation and every `.fa` file of the repository, and prints the counts; `cargo run --release -p fa-formatter --example benchmark` measures the throughput.
- `cargo run -p fa -- migrate-generics [--write] [--verify] [--rust] [--verbose] <files or folders>` (hidden, `packages/cli/src/migrate.rs` over `fa_analyzer::migrate`): moves the parenthesized generic lists to angle brackets ("Generics with angle brackets" and "Generic functions carry their list on the function value" in `design/decisions.md`): declarations (`type Pair<Left, Right>`, `function first = <Item>(…) => …`) and applications told apart from calls by the analysis (`TYPE_EXPR` and `GENERIC_APPLICATION` flags), everything else byte for byte. Each file is analyzed in its program (an analyzer case with `-- file:` sections section by section); without `--write` it lists the files that would change; `--verify` analyzes the migrated files again and prints every diagnostic that differs; `--verbose` lists the calls of capitalized names kept as calls; `--rust` migrates the Fa code of the Rust string literals that parse alone, escapes kept. Run it on a copy until the flag day.
- `cargo fmt --all`: formats all Rust code to project standards (hard tabs).
- `cargo clippy --workspace --all-targets`: lints all Rust code and catches common mistakes.
- `bun fmt`: formats the JavaScript, TypeScript and Svelte code (style in the root `bunfig.toml`); `bun fmt --check` only checks.
- `bun run dev` (from `packages/website`): starts the website, which renders the Markdown pages on each request.
- `bun run build` (same directory): prerenders the whole website into `build/`; it fails on a broken link or anchor. `bun run preview` serves it.
- `bun run check` (same directory): type checks the TypeScript files with `bun check` and the Svelte components with `svelte-check`.
- `bun run check:snippets` (same directory): checks that every Fa snippet of the landing page parses (needs `cargo build -p fa-parser`).
- `bun check` (from `packages/benchmark-typescript`): type checks the benchmark.
- Async benchmarks (`benchmarks/async/`, see `design/async-spike.md`): `cargo run --release -- [n] [current|multi]` from `benchmarks/async` runs the Tokio side (its own Cargo project, not in the workspace). Go is installed user-locally in `~/.local/go`: download the linux-amd64 tarball of the latest version from go.dev/dl (`curl -s 'https://go.dev/VERSION?m=text'` names it), check its SHA-256 against `https://go.dev/dl/?mode=json`, and extract it with `tar -C ~/.local -xzf go<version>.linux-amd64.tar.gz` (replacing `~/.local/go`). Run `~/.local/go/bin/go` (or add `~/.local/go/bin` to `PATH`) with `GOTOOLCHAIN=local`, e.g. `GOTOOLCHAIN=local ~/.local/go/bin/go -C benchmarks/async/go run . [n]` (`GOMAXPROCS=1` for one thread). `cargo run -p fa-codegen-llvm --release --example coroutines [-- switches]` runs the LLVM coroutine spike (`examples/coroutines/`, tested by `tests/coroutines.rs`).

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

### Diagnostics (LSP)
When emitting diagnostics, use `tower_lsp::lsp_types`:
- `Diagnostic`, `DiagnosticSeverity`, `DiagnosticTag`
- `Range`, `Position`
- `DiagnosticRelatedInformation`, `Location`, `LocationLink`
- `Url`, `PublishDiagnosticsParams`

Simple examples:
```rust
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

let range = Range::new(Position::new(0, 4), Position::new(0, 7));
let diagnostic = Diagnostic {
	range,
	severity: Some(DiagnosticSeverity::ERROR),
	message: "Unexpected token".to_string(),
	..Default::default()
};
```

```rust
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, Position, Range};

let range = Range::new(Position::new(2, 0), Position::new(2, 5));
let diagnostic = Diagnostic {
	range,
	severity: Some(DiagnosticSeverity::HINT),
	tags: Some(vec![DiagnosticTag::UNNECESSARY]),
	message: "Unused value".to_string(),
	..Default::default()
};
```

### Svelte & TypeScript
- **Components**: PascalCase filenames for Svelte component and files that export a single Type (e.g., `Home.svelte`, `FancyButton.svelte`, `SomeType.ts`).
- **Routes**: camelCase for other filenames or folders (e.g., `gettingStarted/introduction/hello.ts`).
- **TypeScript**: strict mode enabled, use explicit types where helpful.
- **Formatting**: `bun fmt` formats `.ts`, `.svelte`, JSON, YAML and CSS files with tabs and a 100-column width, and sorts the imports.
- **Tooling**: Bun runs everything (scripts, Vite, route generation). TypeScript 7 is installed as `@typescript/native`; the `typescript` package is the TypeScript 6 API (`@typescript/typescript6`), which SvelteKit, `svelte-check` and the benchmark still need.

### Testing
- Unit tests: colocate in `src/` as modules with `#[cfg(test)]`.
- Integration tests: place in crate-level `tests/` directories.
- Test names: descriptive snake_case that mirrors behavior (`function_calls_no_parameters`, `parses_expression_with_operators`).
- Helper functions: prefix with `test_` or `assert_` to indicate test utilities.
- Gate long-running tests with `#[ignore]` (e.g., `#[test] #[ignore] fn integration_test()`).
- Use `setup_test_directory()` pattern for fixtures requiring temp directories.

## Parser Package Notes (`packages/parser`)

### How It Works
- The entry points are `parse(&str)`, `parse_shared(Arc<str>)` (`packages/parser/src/parsing/parse.rs`) and `parse_single_statement(&str)` (`parse_single_statement.rs`), re-exported at the crate root. They never panic: they return a `TypedSyntaxTree` that owns its source (`Arc<str>`), with `diagnostics: Vec<Diagnostic>` (`diagnostic.rs`: severity, actionable message, span). Code that can't be parsed becomes `Node::Invalid`, and the parser recovers at the next line or closing bracket.
- `tokenize` (`packages/parser/src/tokenize.rs`) converts the input into `Token { kind, start, end }` and normalizes newlines into `TokenKind::Stop` (a `<` not followed by a space opens a generic list, `GenericOpen`, and the `>` that closes it is `GenericClose`, from a stack of the open brackets): a line continues after an opening token and before a chainable or closing token (see the sections of `tokens.rs`). It reports invalid characters and unclosed strings or comments as diagnostics. Comments are dropped, but their spans are kept in `tree.comments` (see `TypedSyntaxTree::documentation`).
- `Context` (`context.rs`) holds the tokens, the position and the tree being built. Report problems with `context.error(...)` / `context.error_at_token(...)`; a second error at the same position is dropped.
- `parse_expression` (`packages/parser/src/parsing/parse_expression.rs`) parses a left expression, then repeatedly parses right expressions (operators, calls, assignments) in `parse_expression_right.rs`, based on `Priority` (`priority.rs`, which follows the order of operations of `documentation/operations/arithmetic.md`). It always returns a node, and stops at the tokens of its `stop_at` slice.
- `Node` in `packages/parser/src/nodes.rs` is the syntax tree node enum (64 bytes; keep it small). Children are node indices; `Node::children()` lists them and `Node::kind_name()` names the variant. Names are `Name` spans into the source (`tree.name(name)` gives the text). Blocks are `Block` nodes holding their `hoisted_symbols`. `TypedSyntaxTree` stores the nodes by index with a parallel `spans` vector, and implements `Display` (`node_to_string` in `typed_syntax_tree.rs`) to render them for tests.
- `cargo test -p fa-parser --test doc_snippets` (or `scripts/check-doc-snippets.sh`) parses every ```fa block of the documentation and of the README, plus truncated and garbled versions of them. A new example of a syntax error in the documentation must be listed in `EXPECTED_ERRORS` of `packages/parser/tests/doc_snippets.rs`.
- `cargo run --release -p fa-parser --bin benchmark -- [file] [repeat]` measures the throughput (the documentation snippets by default).

### Minimal Examples

```rust
use fa_parser::parse;

let tree = parse("a + b");
assert!(tree.diagnostics.is_empty());
assert_eq!(tree.to_string(), "(a + b);");
```

```rust
use fa_parser::parse_single_statement;

let tree = parse_single_statement("function add = (a, b) => a + b");
assert_eq!(tree.to_string(), "function add = (a, b) => (a + b)");
```

### Adding a New Node / Syntax
- **Token (if new syntax):** add/adjust tokenization in `packages/parser/src/tokenize.rs` and `packages/parser/src/tokens.rs` (and `TokenKind::describe`).
- **Node:** add a variant in `packages/parser/src/nodes.rs`, and its children to `Node::for_each_child` and its name to `Node::kind_name`.
- **Parsing:** wire it in `packages/parser/src/parsing/parse_expression.rs` (prefix) or `parse_expression_right.rs` (infix, postfix) or create a helper module and call it from there. Never `panic!` on user input: report a diagnostic that says how to fix the code, and return a node.
- **Priority:** update `packages/parser/src/priority.rs` if the operator needs a new precedence.
- **Rendering:** implement formatting in `packages/parser/src/typed_syntax_tree.rs`.
- **Tests:** add or extend tests under `packages/parser/src/tests/` with the helpers of `tests/helpers.rs` (`assert_statement`, `assert_module`, `assert_has_error`, `assert_error_message`); the ones in `recovery.rs` check diagnostics.

### Practical Advice
- If you add a prefix keyword (e.g. `foo <expr>`), follow `parse_prefix` in `parse_expression` and choose its `Priority` from the order of operations.
- If you add a binary operator, follow the `List!` (flat chains) or `Binary!` pattern in `parse_expression_right` so precedence and associativity work correctly.
- For new grouped constructs, use `parse_delimited` (`parsing/parse_delimited.rs`), which handles separators, missing closing brackets and recovery.
- Every loop over tokens must move forward even on unexpected input; `tests/doc_snippets.rs` checks that garbled code never panics nor hangs.
- Every recursion of the parser, and every link of a chain like `f()()`, counts in `context.depth` (see `parse_expression` and `parse_pattern`): deeper code is reported with `Context::skip_too_deep_code` instead of overflowing the stack, and the tree stays shallow enough for a recursive walk.

## Before Committing
Always run these commands:
```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```
For changes to the JavaScript, TypeScript or Svelte code, also run:
```bash
bun fmt
(cd packages/website && bun run check && bun run check:snippets && bun run build)
```
For changes to the JavaScript runtime (`packages/codegen-js/runtime`), also run `bun test packages/codegen-js/runtime`.
For parser changes, ensure existing tests pass and add new tests for new behaviors.

## Required After Changes
Always run `cargo test` after making code changes and fix any errors or failures before proceeding.

## Commit & Pull Request Guidelines
- Follow the existing convention: emoji prefix plus concise, present-tense summary.
  - Examples: `✨ Add tuple pattern support`, `🐛 Fix function parsing edge case`, `📝 Update installation docs`
- Keep subjects under ~72 characters.
- Include body bullets for context when needed.
- Link relevant issues (if any).
- Note manual test commands for visible/CLI changes.
- Include screenshots or terminal logs for UI changes.
- Small, well-scoped PRs review fastest; split parser and frontend updates when practical.
