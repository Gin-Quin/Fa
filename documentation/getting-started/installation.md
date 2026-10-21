# Installation

:::caution
Fa is in active development and there is no installer yet. You can clone the repository and build from source to try it out. Expect bugs and breaking changes until the first stable release.
:::

## Building from source

Fa's compiler is written in Rust, and its documentation site uses [Bun](https://bun.sh). You will need both.

```bash
git clone https://github.com/your-org/fa.git
cd fa
just install
cargo build --workspace --release
```

The binary is produced at `target/release/fa`. Put it on your `PATH`:

```bash
export PATH="$PWD/target/release:$PATH"
```

Check that it works:

```bash
fa --version
```

## Your first file

```bash
echo 'console.log("Hello, World!")' > hello.fa
fa run hello.fa
```

See [hello world](../guides/hello-world) for the next step.

## Editor support

Fa ships a language server, providing diagnostics, completion, go-to-definition and formatting.

```bash
cargo build -p fa-language-server-protocol --release
```

Point your editor's LSP client at the resulting binary. Fa source files use the `.fa` extension.

## Commands

| Command | What it does |
| --- | --- |
| `fa run <file>` | run a standalone file |
| `fa new package` | scaffold a package |
| `fa new project` | scaffold a project |
| `fa develop` | development server with hot reloading |
| `fa test` | run the tests |
| `fa check` | type-check without producing output |
| `fa build` | compile for the configured platforms |
| `fa preview` | run the compiled application |
| `fa format` | format every source file |
