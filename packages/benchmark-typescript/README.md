# benchmark-typescript

To install dependencies:

```bash
bun install
```

To run the benchmark, which type checks `fixtures/long-file.ts` with the TypeScript API:

```bash
bun run benchmark
```

This fixture has 77 lines and is unrelated to the generated Fa compiler packages.
The harness creates three fresh TypeScript 6 API Programs in one Bun process; it
does not call `bun check` or compare the same program in Fa. The `typescript`
alias keeps the TS6 API used by this harness; `@typescript/native` pins native TS7.
For the matched fresh-process Fa/Bun checker comparison, use the
[compiler audit](../../benchmarks/compiler/README.md). Bun's built-in checker
version comes from `process.versions.typescript`, independently of the API alias.

To type check this package:

```bash
bun check
```

This project was created using `bun init` in bun v1.3.5. [Bun](https://bun.com) is a fast all-in-one JavaScript runtime.
