# Hot reloading

During development, Fa compiles to JavaScript and runs in [Bun](https://bun.sh). Because JavaScript is JIT-compiled, this gives near-instant feedback: far faster than recompiling a native binary on every keystroke.

Start a development session from the root of your package or workspace:

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

In a [workspace](/architecture/workspaces), `fa develop` starts every package that implements it. Name a package to develop only that one:

```bash
fa develop api
```

## What is preserved

A reload replaces the changed modules and keeps the program running. [Reactive](/advanced/reactivity) state survives, so a user interface keeps its state across an edit.

State is lost when the shape of the program changes in a way that cannot be reconciled: a changed type, a renamed export, or a changed module structure. Fa restarts the program in that case, and tells you why.

## Configuration

Set the development options in the `develop` field of `package.fa` (or of `workspace.fa`, for every package):

```fa
-- package.fa
export = {
	name = "farm"
	develop = {
		port = 3000
		open = true -- open a browser on start
		hotReload = false -- restart instead of reloading modules
	}
}
```

See [configuration](/architecture/configuration#develop).

## Developing against another target

`fa develop` uses the `bun` target by default, because it starts fastest. When you need to exercise behavior that differs per target, ask for it:

```bash
fa develop --target=native
fa develop --target=web
```

Native reloads are slower, since each change recompiles.

:::caution
Development builds are unoptimized and keep debug information. Always measure performance on a `fa build` output, never on `fa develop`. See [performance](/guides/performance).
:::
