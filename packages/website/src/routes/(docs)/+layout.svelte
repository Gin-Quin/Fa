<script lang="ts">
	import { afterNavigate } from "$app/navigation";
	import type { Snippet } from "svelte";

	import Header from "#lib/components/Header.svelte";
	import Icon from "#lib/components/Icon.svelte";
	import Sidebar from "#lib/docs/Sidebar.svelte";

	import type { LayoutData } from "./$types";

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	let menuOpen = $state(false);

	afterNavigate(() => (menuOpen = false));
</script>

<Header>
	{#snippet before()}
		<button
			class="menu-button"
			type="button"
			aria-label={menuOpen ? "Close the menu" : "Open the menu"}
			aria-expanded={menuOpen}
			onclick={() => (menuOpen = !menuOpen)}
		>
			<Icon name={menuOpen ? "close" : "menu"} size={20} />
		</button>
	{/snippet}
</Header>

<div class="layout">
	<aside class="sidebar" class:open={menuOpen}>
		<Sidebar navigation={data.navigation} onnavigate={() => (menuOpen = false)} />
	</aside>
	{#if menuOpen}
		<div class="scrim" role="presentation" onclick={() => (menuOpen = false)}></div>
	{/if}
	<div class="content">
		{@render children()}
	</div>
</div>

<style>
	.layout {
		display: grid;
		grid-template-columns: var(--width-sidebar) minmax(0, 1fr);
		min-height: calc(100vh - var(--height-header));
	}

	.sidebar {
		position: sticky;
		top: var(--height-header);
		height: calc(100vh - var(--height-header));
		overflow-y: auto;
		overscroll-behavior: contain;
		border-right: 1px solid var(--color-border);
		background: var(--color-background);
		scrollbar-width: thin;
		scrollbar-color: var(--color-border-strong) transparent;
	}

	.menu-button {
		display: none;
		place-items: center;
		width: 2.25rem;
		height: 2.25rem;
		margin-left: calc(-1 * var(--space-2));
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
	}

	.scrim {
		display: none;
	}

	@media (max-width: 960px) {
		.layout {
			grid-template-columns: minmax(0, 1fr);
		}

		.menu-button {
			display: grid;
		}

		.sidebar {
			position: fixed;
			top: var(--height-header);
			left: 0;
			bottom: 0;
			z-index: var(--layer-overlay);
			width: min(var(--width-sidebar), 85vw);
			height: auto;
			transform: translateX(-100%);
			transition: transform var(--duration-normal) var(--ease-out);
		}

		.sidebar.open {
			transform: none;
			box-shadow: var(--shadow-lg);
		}

		.scrim {
			display: block;
			position: fixed;
			inset: var(--height-header) 0 0;
			z-index: calc(var(--layer-overlay) - 1);
			background: var(--color-overlay);
		}
	}
</style>
