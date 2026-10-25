<script lang="ts">
	import { page } from "$app/state";

	import Icon from "#lib/components/Icon.svelte";

	import type { NavigationSection } from "./types.ts";

	let { navigation, onnavigate }: { navigation: NavigationSection[]; onnavigate?: () => void } =
		$props();

	const current = $derived(page.url.pathname.slice(1));

	// every section is open by default; the reader can collapse the ones they don't need
	let collapsed = $state<Record<string, boolean>>({});
</script>

<nav aria-label="Documentation">
	{#each navigation as section (section.title)}
		{@const isCollapsed =
			collapsed[section.title] && !section.pages.some((item) => item.slug === current)}
		<div class="section">
			<button
				type="button"
				class="section-title"
				aria-expanded={!isCollapsed}
				onclick={() => (collapsed[section.title] = !isCollapsed)}
			>
				{section.title}
				<Icon name="chevronDown" size={14} class="chevron" />
			</button>
			{#if !isCollapsed}
				<ul>
					{#each section.pages as item (item.slug)}
						<li>
							<a
								href="/{item.slug}"
								aria-current={item.slug === current ? "page" : undefined}
								onclick={onnavigate}
							>
								{item.title}
							</a>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/each}
</nav>

<style>
	nav {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		padding: var(--space-6) var(--space-4) var(--space-8);
	}

	.section-title {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: var(--space-1) var(--space-3);
		font-size: var(--text-xs);
		font-weight: var(--weight-semibold);
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
		color: var(--color-text);
	}

	.section-title :global(.chevron) {
		color: var(--color-text-subtle);
		transition: transform var(--duration-normal) var(--ease-out);
	}

	.section-title[aria-expanded="false"] :global(.chevron) {
		transform: rotate(-90deg);
	}

	ul {
		list-style: none;
		margin-top: var(--space-2);
		padding: 0;
		border-left: 1px solid var(--color-border);
		margin-left: var(--space-3);
	}

	a {
		position: relative;
		display: block;
		margin-left: -1px;
		padding: 0.35rem var(--space-3) 0.35rem var(--space-4);
		border-left: 1px solid transparent;
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		transition:
			color var(--duration-fast),
			border-color var(--duration-fast);
	}

	a:hover {
		color: var(--color-text);
		border-left-color: var(--color-border-strong);
	}

	a[aria-current="page"] {
		color: var(--color-accent-text);
		font-weight: var(--weight-medium);
		border-left-color: transparent;
	}

	a[aria-current="page"]::before {
		content: "";
		position: absolute;
		left: -1px;
		top: 0.3rem;
		bottom: 0.3rem;
		width: 2px;
		border-radius: 2px;
		background: var(--gradient-brand-vertical);
	}
</style>
