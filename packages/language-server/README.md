# fa-language-server

The language server of Fa (LSP 3.17, over stdin and stdout), built with `tower-lsp`.

What it provides today, from the parser alone:

- diagnostics: the syntax errors, published 50 ms after the last edit,
- document symbols: functions, types (with their fields, methods and variants), namespaces, values, tests, nested,
- folding ranges: blocks, brackets on several lines, comments,
- semantic tokens: keywords, strings (and their interpolations), numbers, comments, operators, declarations, types (PascalCase names), functions, properties,
- selection ranges,
- completion of the keywords.

Hover, go-to-definition, completion of names and type errors come with the analyzer (`packages/analyzer`): the handlers are wired to the `AnalysisBackend` trait of `src/backend.rs`, which only parses for now.

## Build

```bash
cargo build -p fa-language-server --release
```

The binary is `target/release/fa-language-server`. Put it on your `PATH`, or use its absolute path in the configurations below. It takes no arguments.

## Editors

### VS Code

VS Code needs an extension to start a language server. The smallest one is two files in a folder of `~/.vscode/extensions/` (run `npm install vscode-languageclient` in it):

`package.json`:

```json
{
	"name": "fa",
	"version": "0.0.1",
	"engines": { "vscode": "^1.75.0" },
	"main": "./extension.js",
	"activationEvents": ["onLanguage:fa"],
	"contributes": {
		"languages": [{ "id": "fa", "extensions": [".fa"] }]
	},
	"dependencies": { "vscode-languageclient": "^9.0.0" }
}
```

`extension.js`:

```js
const { LanguageClient } = require("vscode-languageclient/node");

let client;

exports.activate = () => {
	client = new LanguageClient(
		"fa",
		"Fa",
		{ command: "fa-language-server" },
		{ documentSelector: [{ language: "fa" }] },
	);
	return client.start();
};

exports.deactivate = () => client?.stop();
```

Semantic highlighting is on by default in VS Code (`editor.semanticHighlighting.enabled`).

### Neovim

With Neovim 0.11 or later:

```lua
vim.filetype.add({ extension = { fa = "fa" } })

vim.lsp.config("fa", {
	cmd = { "fa-language-server" },
	filetypes = { "fa" },
	root_markers = { "workspace.fa", "package.fa", ".git" },
})
vim.lsp.enable("fa")
```

With `nvim-lspconfig` on older versions:

```lua
vim.filetype.add({ extension = { fa = "fa" } })

local configs = require("lspconfig.configs")
if not configs.fa then
	configs.fa = {
		default_config = {
			cmd = { "fa-language-server" },
			filetypes = { "fa" },
			root_dir = require("lspconfig.util").root_pattern("workspace.fa", "package.fa", ".git"),
		},
	}
end
require("lspconfig").fa.setup({})
```

### Helix

In `~/.config/helix/languages.toml`:

```toml
[language-server.fa-language-server]
command = "fa-language-server"

[[language]]
name = "fa"
scope = "source.fa"
file-types = ["fa"]
roots = ["workspace.fa", "package.fa"]
comment-token = "--"
indent = { tab-width = 4, unit = "\t" }
language-servers = ["fa-language-server"]
```

Helix highlights with tree-sitter grammars, not with semantic tokens: there is no Fa grammar yet, so the code is not colored, but the diagnostics, symbols and selection ranges work.

## Positions

The parser's spans are byte offsets; LSP columns are UTF-16 code units unless the client supports UTF-8 (`general.positionEncodings`, LSP 3.17), which the server then chooses. `src/line_index.rs` converts between them, and the documents are kept in sync incrementally (`TextDocumentSyncKind::INCREMENTAL`).

## Architecture

- `server.rs`: the LSP handlers. They send every notification and query to the analysis thread, as a message, before any `.await`, so that the order is kept.
- `worker.rs`: the analysis thread. It owns the documents and the backend, applies the edits, analyzes a document 50 ms after its last edit (or right away when a query needs it), and publishes the diagnostics.
- `backend.rs`: the `AnalysisBackend` trait (byte offsets, no LSP positions) and `ParseBackend`, its parse-only implementation.
- `features/`: symbols, folding, semantic tokens, selection ranges, completion, and the conversion of the diagnostics.

To plug in the analyzer, implement `AnalysisBackend` over its `Session` (`set_document` calls `Session::update` and returns the files it re-analyzed, `tree` returns `Session::tree`, `hover`, `definition` and `completions` call the queries of the same name, `cancel_flag` returns `Session::cancel_flag`), and pass it to `FaLanguageServer::with_backend` in `server::service`.

## Tests

```bash
cargo test -p fa-language-server
```

The unit tests are next to the code; `tests/lsp.rs` drives the `LspService` in process with JSON-RPC messages.
