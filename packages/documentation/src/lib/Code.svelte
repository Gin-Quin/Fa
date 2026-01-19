<script lang="ts">
	import type { Highlighter } from "shiki";

	import { getHighlighter, shikiThemes } from "./shikiHighlighter";

	let { code = "", lang = "fa" } = $props();
	let html = $state("");
	let highlighter = $state<Highlighter | null>(null);

	$effect(() => {
		let cancelled = false;

		(async () => {
			let loaded: Highlighter;

			try {
				loaded = await getHighlighter();
			} catch {
				return;
			}

			if (cancelled) {
				return;
			}

			highlighter = loaded;
		})();

		return () => {
			cancelled = true;
		};
	});

	$effect(() => {
		if (!highlighter) {
			return;
		}

		const normalizedLang =
			lang === "ts"
				? "typescript"
				: lang === "sh"
					? "bash"
					: lang === "rs"
						? "rust"
						: lang === "js"
							? "javascript"
							: lang;

		html = highlighter.codeToHtml(code, {
			lang: normalizedLang,
			themes: {
				light: shikiThemes.light,
				dark: shikiThemes.dark,
			},
		});
	});
</script>

<div class="fa-code" aria-label="Code sample">
	{#if html}
		{@html html}
	{:else}
		<pre><code>{code}</code></pre>
	{/if}
</div>

<style>
	.fa-code :global(pre) {
		margin: 0;
		padding: 1rem 1.25rem;
		border-radius: 12px;
		overflow: auto;
	}
</style>
