<script lang="ts">
	import type { TocEntry } from "./types.ts";

	let { entries }: { entries: TocEntry[] } = $props();

	let active = $state("");

	// highlights the last heading that went above the top quarter of the screen
	$effect(() => {
		const headings = entries
			.map((entry) => document.getElementById(entry.id))
			.filter((element): element is HTMLElement => element !== null);

		const update = () => {
			const limit = window.innerHeight * 0.25;
			let current = headings[0]?.id ?? "";
			for (const heading of headings) {
				if (heading.getBoundingClientRect().top <= limit) {
					current = heading.id;
				}
			}
			active = current;
		};

		update();
		window.addEventListener("scroll", update, { passive: true });
		return () => window.removeEventListener("scroll", update);
	});
</script>

{#if entries.length > 0}
	<nav aria-label="On this page">
		<p class="title">On this page</p>
		<ul>
			{#each entries as entry (entry.id)}
				<li class:nested={entry.depth === 3}>
					<a href="#{entry.id}" class:active={active === entry.id}>{entry.text}</a>
				</li>
			{/each}
		</ul>
	</nav>
{/if}

<style>
	.title {
		margin-bottom: var(--space-3);
		font-size: var(--text-xs);
		font-weight: var(--weight-semibold);
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
		color: var(--color-text);
	}

	ul {
		list-style: none;
		padding: 0;
		display: grid;
		gap: var(--space-1);
	}

	.nested {
		padding-left: var(--space-3);
	}

	a {
		display: block;
		padding-block: 0.2rem;
		font-size: var(--text-sm);
		line-height: 1.45;
		color: var(--color-text-subtle);
		transition: color var(--duration-fast);
	}

	a:hover {
		color: var(--color-text);
	}

	a.active {
		color: var(--color-accent-text);
	}
</style>
