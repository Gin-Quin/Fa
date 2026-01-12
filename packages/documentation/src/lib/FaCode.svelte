<script lang="ts">
	import type { Highlighter } from "shiki";

	import { getHighlighter, shikiThemes } from "./shikiHighlighter";

	let { code = "" } = $props();
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

		html = highlighter.codeToHtml(code, {
			lang: "fa",
			themes: {
				light: shikiThemes.light,
				dark: shikiThemes.dark,
			},
		});
	});
</script>

<div class="fa-code" aria-label="Fa code sample">
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
