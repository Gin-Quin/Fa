# benchmark-typescript

To install dependencies:

```bash
bun install
```

To run the benchmark, which type checks `fixtures/long-file.ts` with the TypeScript API:

```bash
bun run benchmark
```

TypeScript 7 doesn't ship an API yet, so the `typescript` package is the TypeScript 6 API (`@typescript/typescript6`). To type check this package:

```bash
bun check
```

This project was created using `bun init` in bun v1.3.5. [Bun](https://bun.com) is a fast all-in-one JavaScript runtime.
