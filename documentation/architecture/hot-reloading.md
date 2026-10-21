# Hot reloading

During development, Fa compiles to JavaScript and runs in [Bun](https://bun.sh). Because JavaScript is JIT-compiled, this gives near-instant feedback — far faster than recompiling a native binary on every keystroke.

Start a development session from the root of your package or project:

```bash
fa develop
```

This will:

- compile your code to JavaScript,
- run it in Bun,
- watch your source files,
- recompile and reload the affected modules when a file changes,
- re-run the tests that cover what changed.

## Targeting one package

In a [project](./projects), name a package to develop only that one:

```bash
fa develop api
```

## What is preserved

A reload replaces the changed modules and keeps the program running. [Reactive](../advanced/reactivity) state survives, so a user interface keeps its state across an edit.

State is lost when the shape of the program changes in a way that cannot be reconciled: a changed type, a renamed export, or a changed module structure. Fa restarts the program in that case, and tells you why.

## Configuration

Set development options in `package.fab`:

```
develop = {
  port = 3000
  open = true
  hotReload = false -- restart instead of reloading modules
}
```

See [configuration](./configuration).

## Developing against another platform

`fa develop` uses the `bun` platform because it starts fastest. When you need to exercise behaviour that differs per target, ask for it:

```bash
fa develop --platform native
fa develop --platform web
```

Native reloads are slower, since each change recompiles.

:::caution
Development builds are unoptimised and keep debug information. Always measure performance on a `fa build` output, never on `fa develop`. See [performance](../guides/performance).
:::
