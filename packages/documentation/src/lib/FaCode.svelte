<script lang="ts">
	import {
		createHighlighter,
		type Highlighter,
		type ThemeRegistrationAny,
	} from "shiki";
	import andromeeda from "@shikijs/themes/andromeeda";
	import catppuccinLatte from "@shikijs/themes/catppuccin-latte";
	import faGrammar from "../shiki/fa.tmLanguage.json";

	const themeDark = andromeeda;
	const themeLight = catppuccinLatte;

	const highlighterPromise = createHighlighter({
		themes: [themeDark, themeLight],
		langs: [faGrammar],
	});

	let { code = "" } = $props();
	let html = $state("");
	let highlighter = $state<Highlighter | null>(null);

	$effect(() => {
		let cancelled = false;

		(async () => {
			let loaded: Highlighter;

			try {
				loaded = await highlighterPromise;
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
				light: themeLight,
				dark: themeDark,
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
