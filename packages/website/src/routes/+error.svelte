<script lang="ts">
	import { page } from "$app/state";

	import Header from "#lib/components/Header.svelte";
</script>

<svelte:head>
	<title>{page.status} · Fa</title>
</svelte:head>

<Header />

<main class="container">
	<p class="status gradient-text">{page.status}</p>
	<h1>{page.status === 404 ? "This page does not exist" : page.error?.message}</h1>
	<p>It may have moved while the documentation was reorganized.</p>
	<div class="actions">
		<a class="button button-primary" href="/">Home</a>
		<a class="button button-secondary" href="/getting-started/introduction">Documentation</a>
	</div>
</main>

<style>
	main {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		padding-block: var(--space-10);
		text-align: center;
	}

	.status {
		font-size: var(--text-hero);
		font-weight: var(--weight-bold);
		letter-spacing: var(--tracking-tight);
		line-height: 1;
	}

	h1 {
		font-size: var(--text-2xl);
		letter-spacing: var(--tracking-snug);
	}

	p {
		color: var(--color-text-muted);
	}

	.actions {
		display: flex;
		gap: var(--space-3);
		margin-top: var(--space-4);
	}
</style>
