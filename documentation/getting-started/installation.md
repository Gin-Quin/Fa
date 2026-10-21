# Installation

:::caution
Fa is in active development and there is no installer yet. The `fa` command-line tool described on this page is **not built yet**: today, the repository contains the parser, the codebase parser and a first version of the language server. Expect bugs and breaking changes until the first stable release.
:::

## Building from source

Fa's toolchain is written in Rust, and its documentation site uses [Bun](https://bun.sh). You will need both.

```bash
git clone https://github.com/Gin-Quin/Fa.git
cd Fa
just install
cargo build --workspace --release
```

The workspace currently builds these crates:

| Crate | Folder | What it is |
| --- | --- | --- |
| `fa-parser` | `packages/parser` | the parser, with a binary that prints the parsed tree of a file |
| `fa-codebase-parser` | `packages/codebase-parser` | resolves the files and folders of a codebase into modules |
| `fa-language-server-protocol` | `packages/language-server` | the language server, in its early stages |

You can already parse a file:

```bash
cargo run -p fa-parser --release -- path/to/file.fa
```

## Your first file

Once the `fa` command exists, running a standalone file will look like this:

```bash
echo 'console.log("Hello, World!")' > hello.fa
fa run hello.fa
```

See [hello world](/guides/hello-world) for the next step.

## Editor support

Fa will ship a language server, providing diagnostics, completion, go-to-definition and formatting. Its first version can be built with:

```bash
cargo build -p fa-language-server-protocol --release
```

Point your editor's LSP client at the resulting binary. Fa source files use the `.fa` extension.

## Commands

These are the commands of the `fa` tool:

| Command | What it does |
| --- | --- |
| `fa run <file>` | run a standalone file |
| `fa new package <name>` | scaffold a [package](/architecture/packages) |
| `fa new workspace` | scaffold a [workspace](/architecture/workspaces) |
| `fa develop` | start a development server with [hot reloading](/architecture/hot-reloading) |
| `fa test` | run the [tests](/architecture/testing) (`--watch`, `--coverage`, `--target=...`) |
| `fa check` | type-check without producing output |
| `fa format` | format every source file |
| `fa build` | compile for the configured targets (`--target=...` to pick one) |
| `fa preview` | run the compiled application |
| `fa update` | update the locked [dependencies](/architecture/dependencies) |
