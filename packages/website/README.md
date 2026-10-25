# Fa website

The website of Fa: a landing page, and the documentation rendered from the Markdown files of the [`documentation/`](../../documentation) folder.

It is a [SvelteKit](https://svelte.dev/docs/kit) site, prerendered into static files.

## Commands

```bash
bun install
bun run dev              # http://localhost:8000
bun run build            # prerenders the site into `build/`
bun run preview          # serves the build
bun run check            # type checks the TypeScript files and the Svelte components
bun run check:snippets   # checks that the Fa snippets of the landing page parse
```

## How it works

- `src/lib/docs/navigation.ts` lists the pages of the documentation, in the order of the sidebar. Every Markdown file must be listed: the build fails otherwise.
- `src/lib/docs/markdown.server.ts` renders the Markdown at build time, with [unified](https://unifiedjs.com): GitHub-flavored Markdown, `:::tip` callouts, heading anchors, and code highlighted by [Shiki](https://shiki.style).
- `src/lib/highlight/fa.tmLanguage.json` is the grammar of Fa used for highlighting.
- `src/lib/styles/tokens.css` is the design system: every color, size, radius, shadow and duration is a CSS variable, with a dark and a light theme.
- `src/lib/landing/` holds the sections of the landing page. Their Fa code is in `snippets.ts`, checked by `bun run check:snippets`.
- `/search.json` is the search index, prerendered from every page and loaded the first time the search is opened (<kbd>⌘K</kbd> or <kbd>/</kbd>).
