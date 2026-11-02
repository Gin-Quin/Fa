# Compiler architecture

The decisions that every crate of the Fa toolchain follows. The language itself is specified in `documentation/`; the implementation briefs derived from it are in `design/implementation/`.

## Pipeline

```
source ──tokenize──▶ tokens ──parse──▶ TypedSyntaxTree ──analyze──▶ Analysis ──┬─▶ JavaScript (fa-codegen-js)
                                                                               └─▶ LLVM IR ─▶ native / wasm (fa-codegen-llvm)
```

Exactly three passes over the source of a file: tokenizing, parsing, analyzing. Analyzing resolves names, infers types, infers and checks effects, tracks narrowing and reference validity, and emits every diagnostic, **in one walk**. The backends then each walk the analyzed tree once.

## Crates

| Crate | Path | Role |
| --- | --- | --- |
| `fa-parser` | `packages/parser` | tokenizer, parser, `TypedSyntaxTree`, parse diagnostics |
| `fa-analyzer` | `packages/analyzer` | the single-pass checker: names, types, effects, flow facts, diagnostics |
| `fa-interpreter` | `packages/interpreter` | the compile-time interpreter: values, the lowered code the analyzer gives it, evaluation with limits (`static`, `@` calls, configuration files) |
| `fa-codebase-parser` | `packages/codebase-parser` | file system modules: the module graph of a standalone file, a package or a workspace |
| `fa-codegen-js` | `packages/codegen-js` | Fa → JavaScript (ES2022 modules) plus a small runtime, and `.d.ts` declarations so that JavaScript and TypeScript code can import Fa modules |
| `fa-codegen-llvm` | `packages/codegen-llvm` | Fa → LLVM IR → native executables and WebAssembly |
| `fa` | `packages/cli` | the `fa` command (`check`, `build`, `run`, `test`, …) |
| `fa-language-server` | `packages/language-server` | the LSP server (tower-lsp) |

No crate depends on a backend except `fa` and the benchmarks. The analyzer does not depend on `tower-lsp`: diagnostics have their own type, converted to LSP types in the language server.

## Data model

- `TypedSyntaxTree` keeps nodes in a `Vec<Node>` addressed by index, with a parallel `Vec<SourceSpan>`. The analyzer never rewrites nodes.
- The analysis stores its results in **side tables indexed by node index**, allocated once with the node count:
  - `types: Vec<TypeId>`: the type of every expression node (`TypeId::NONE` for nodes without one),
  - `resolutions`: the symbol each identifier node designates,
  - instantiations of generic calls (the type arguments chosen at each call node), used by the LLVM backend to monomorphize,
  - per-function facts: inferred effects, purity, escaping parameters.
- Types are **interned** in a `TypeStore`: a `TypeId` is a `u32` index; structurally equal types have the same id, so type equality is an integer comparison. Literal types, ranged integers, unions, function types with effects and reference types (`T in path`) are all variants of the same table.
- Symbols live in a `SymbolTable` (`SymbolId` = `u32`), with their declaration node, kind and type.
- Diagnostics: `{ severity, code, message, span, related: Vec<(span, message)> }`. Messages follow the documentation's rule: written to be acted upon.

## The single analysis walk

- **Bidirectional checking**: `check(node, expected: Option<TypeId>) -> TypeId`. The expected type flows down (object literals, lambdas, number literals adapt to it), the inferred type flows up.
- **Hoisting without a pre-pass over bodies**: the parser already records the hoisted symbols of each block. Entering a block declares them as *pending*; the first lookup of a pending symbol analyzes its declaration on demand (memoized), and a lookup of a symbol being analyzed reports a cycle. So every declaration is analyzed exactly once, in the order needed.
- **Flow facts in the same walk**: narrowing, exact literals and ranges, and the validity of references are a *flow state* carried along the walk. Branches fork it and join it; loops re-run the flow transfer of their body until the state stops changing (only the flow transfer, not the type inference, which is memoized per node).
- **Effects**: a function with a single-expression body gets its effects inferred while its body is analyzed; a block body is checked against its declared effects. Calls add the callee's effects (renamed to the arguments) to the current function.
- **Multi-file**: each file is parsed independently (in parallel); the analyzer resolves names across files through the module graph and analyzes another module's declaration on demand, the same way as hoisted symbols.
- The language server re-runs the parse and the analysis of the changed file and of its dependents; everything is designed so a full re-analysis of a file is fast (no global passes).

## Performance principles (user requirements, 2026-11-02)

- **Fast on big projects, using every core**: parsing, analysis and code generation run in parallel with Rayon. Files are parsed in parallel; modules are analyzed in parallel by dependency layers (a module once the interfaces it reads are known); each backend generates modules in parallel (one JavaScript module per Fa file; LLVM modules per file or per group, then linked).
- **Memory independent of the project size**: the toolchain streams. It keeps the interfaces of analyzed modules (exported signatures, types, summaries) and the module being processed, and drops a module's tree and side tables once its code is generated; outputs are written as they are produced, never accumulated. Only the language server keeps the open files. Checking validity (`fa check`, `fa format --check`, the language server) stays under 512 MB whatever the project.
- **Minimal output (user requirement, 2026-11-03)**: a Fa value is never bigger, and an operation never costs more, than what a C programmer would write by hand with the same type information. The strict typing exists to generate *less*: exact types give unboxed, tag-free layouts (`T?` of a pointer is a nullable pointer, a narrowed union has no tag check, a field stores its proven range's width), value semantics and the alias rule remove copies and reloads, ownership allows arenas and bulk frees, and code specialized on the exact type replaces generic runtime paths. Every benchmark where Fa executes more instructions or uses more memory than the reference (Rust, Goose) is a bug to explain and fix.
- **Code quality**: as few passes and loops as possible (one analysis walk, one generation walk), little mutation of data (side tables written once per node, interned immutable types), no copies of trees.

## Backends

- Both backends walk the analyzed tree directly; there is no shared intermediate representation at first. Desugarings that both need (for example the lowering of `when` patterns) live in small helpers of `fa-analyzer` so they are written once.
- **JavaScript**: generics are erased; integers are numbers with overflow checks against `2^53 − 1` (64-bit and wider types are `BigInt`s), as the documentation says. The runtime is a tree-shakeable ES module, inlined only for what a program uses. Output must be readable and not surprise a JavaScript developer.
- **LLVM**: generics are monomorphized from the instantiation side table. Integer overflow uses the `*.with.overflow` intrinsics only where the analyzer could not prove the absence of overflow. Size matters: `-Oz`-friendly code, a tiny runtime, dead code elimination, no unwinding tables.

## Testing

- Every crate has unit tests; the toolchain has end-to-end tests: a directory of Fa programs with their expected output (or expected diagnostics), run through `fa check`, the JavaScript backend (executed with Bun or Node) and the native backend, and compared.
- Every ```fa block of the documentation must parse; the ones that are complete programs must also check.

## Benchmarks

`benchmarks/` holds the same programs in Fa and in Rust (and TypeScript where useful), and a runner that builds them, checks they print the same output, and reports run time and binary size: Fa native vs Rust `-O`, Fa wasm, Fa JavaScript on Bun.

## Scope of the first version

Everything the documentation describes, except `async`/`await`/`start` and tasks. Implemented in this order:

1. Core: numbers with overflow and range checks, booleans, strings and interpolation, declarations, functions, control flow, objects and types, unions and enumerations, `when`, optionals, errors, arrays, `console`, tests and `assert`, a standalone file.
2. Generics, methods, tuples, ranges, destructuring, maps and sets, narrowing, effects checking, reference checking, multi-file modules, requirements.
3. Pools, relations, columnars, growing collections, reactivity, compile-time evaluation (`static`, `@` operations), foreign code, `for parallel` and workers, configuration files, packages and workspaces.
