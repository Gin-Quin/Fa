<script lang="ts">
	import type { Snippet } from "svelte";

	import { reveal } from "./reveal.ts";

	let {
		eyebrow,
		title,
		children,
		align = "center",
	}: { eyebrow: string; title: Snippet; children?: Snippet; align?: "center" | "left" } = $props();
</script>

<header class="section-header {align}" {@attach reveal()}>
	<p class="eyebrow"><span class="dot"></span>{eyebrow}</p>
	<h2>{@render title()}</h2>
	{#if children}
		<p class="lead">{@render children()}</p>
	{/if}
</header>

<style>
	.section-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		max-width: 46rem;
		margin-bottom: var(--space-8);
	}

	.center {
		align-items: center;
		margin-inline: auto;
		text-align: center;
	}

	.eyebrow {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: var(--weight-medium);
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
		color: var(--color-accent-text);
	}

	.dot {
		width: 6px;
		height: 6px;
		border-radius: 1px;
		background: var(--gradient-brand);
		transform: rotate(45deg);
	}

	h2 {
		font-family: var(--font-display);
		font-size: var(--text-4xl);
		font-weight: var(--weight-bold);
		letter-spacing: var(--tracking-tight);
		line-height: var(--leading-tight);
		color: var(--color-text);
		text-wrap: balance;
	}

	.lead {
		font-size: var(--text-lg);
		line-height: 1.6;
		color: var(--color-text-muted);
		text-wrap: pretty;
	}
</style>
