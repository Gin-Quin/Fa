# Repository Guidelines

## Project Structure & Module Organization
Fa is organized as a Rust workspace rooted in `Cargo.toml`, with core crates under `packages/` (`parser`, `analyzer`, `parser-v2`, `codebase-parser`, `language-server-protocol`). Shared experimental code lives in `sandbox/`, and built artifacts land in `target/`. The developer-facing site is under `packages/documentation` (SvelteKit) and sources Markdown from the top-level `documentation/` directory. Keep language assets grouped by package; avoid duplicating source across crates.

## Build, Test, and Development Commands
- `just install`: installs Bun or Cargo dependencies package by package.
- `cargo build --workspace`: compiles all Rust crates; use `-p <crate>` for granular builds.
- `cargo check --workspace`: fast type-check for Rust code before committing.
- `cargo test --workspace`: runs all Rust test suites (see `packages/parser/tests` for examples).
- `bun run dev` (from `packages/documentation`): starts the docs site with live route generation.
- `bun run build` (same directory): produces a static documentation bundle.

## Coding Style & Naming Conventions
Rust code follows edition 2024 defaults—run `cargo fmt --all` and `cargo clippy --workspace --all-targets` before sending a PR. Modules use snake_case files, CamelCase types, and snake_case functions. Svelte/TypeScript files in `packages/documentation/src` use PascalCase component names and kebab-case routes. When editing Nim or Fa placeholders in `packages/library`, keep filenames lowercase and mirror the module folder name (e.g., `Vector/module.fa`).

## Testing Guidelines
Prefer unit tests colocated in each crate under `src/` or `tests/`. Name Rust tests with descriptive snake_case that mirrors the behavior (`parses_function_call`). When adding integration tests, create modules under `tests/` and gate long-running cases behind `#[ignore]`. Aim to cover new parser and analyzer behaviors with at least one focused test; rerun `cargo test --workspace` before review. For documentation features, add a reproduction page under `documentation/` and exercise it manually in `bun run dev`.

## Commit & Pull Request Guidelines
Follow the existing convention: emoji prefix plus concise, present-tense summary (e.g., `✨ Add tuple pattern support`). Keep subjects under ~72 characters and include body bullets for context when needed. Every PR should link the relevant issue (if any), note manual test commands, and include screenshots or terminal logs for visible or CLI-facing changes. Small, well-scoped PRs review fastest; split parser and frontend updates when practical.
`
