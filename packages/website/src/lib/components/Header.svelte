<script lang="ts">
	import { page } from "$app/state";
	import type { Snippet } from "svelte";

	import { search } from "#lib/search/search.svelte.ts";

	import GithubIcon from "./GithubIcon.svelte";
	import Icon from "./Icon.svelte";
	import Logo from "./Logo.svelte";
	import ThemeToggle from "./ThemeToggle.svelte";

	let { before }: { before?: Snippet } = $props();

	const LINKS = [
		{ label: "Docs", href: "/getting-started/introduction", match: "" },
		{ label: "Reference", href: "/reference/language-reference", match: "/reference" },
		{ label: "Guides", href: "/guides/hello-world", match: "/guides" },
	];

	let scrolled = $state(false);

	const isDocumentation = $derived(page.url.pathname !== "/");

	function isActive(match: string) {
		const path = page.url.pathname;
		if (match) {
			return path.startsWith(match);
		}
		return isDocumentation && !LINKS.some((link) => link.match && path.startsWith(link.match));
	}
</script>

<svelte:window onscroll={() => (scrolled = window.scrollY > 8)} />

<header class:scrolled class:docs={isDocumentation}>
	<div class="inner">
		{#if before}{@render before()}{/if}

		<a class="home" href="/" aria-label="Fa, home page">
			<Logo />
		</a>

		<nav aria-label="Main">
			{#each LINKS as link (link.href)}
				<a href={link.href} class:active={isActive(link.match)}>{link.label}</a>
			{/each}
		</nav>

		<div class="actions">
			<button class="search" type="button" onclick={() => search.show()}>
				<Icon name="search" size={16} />
				<span class="search-label">Search</span>
				<kbd>⌘K</kbd>
			</button>
			<ThemeToggle />
			<a
				class="icon-link"
				href="https://github.com/Gin-Quin/Fa"
				target="_blank"
				rel="noopener noreferrer"
				aria-label="Fa on GitHub"
			>
				<GithubIcon />
			</a>
		</div>
	</div>
</header>

<style>
	header {
		position: sticky;
		top: 0;
		z-index: var(--layer-header);
		height: var(--height-header);
		border-bottom: 1px solid transparent;
		transition:
			background-color var(--duration-normal),
			border-color var(--duration-normal);
	}

	header.scrolled,
	header.docs {
		background: color-mix(in srgb, var(--color-background) 90%, transparent);
		backdrop-filter: saturate(1.6) blur(14px);
		-webkit-backdrop-filter: saturate(1.6) blur(14px);
		border-bottom-color: var(--color-border);
	}

	.inner {
		display: flex;
		align-items: center;
		gap: var(--space-5);
		height: 100%;
		max-width: var(--width-page);
		margin-inline: auto;
		padding-inline: var(--gutter);
	}

	header.docs .inner {
		max-width: none;
	}

	.home {
		flex-shrink: 0;
	}

	nav {
		display: flex;
		gap: var(--space-1);
	}

	nav a {
		position: relative;
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-full);
		font-size: var(--text-sm);
		font-weight: var(--weight-medium);
		color: var(--color-text-muted);
		transition: color var(--duration-fast);
	}

	nav a:hover,
	nav a.active {
		color: var(--color-text);
	}

	nav a.active::after {
		content: "";
		position: absolute;
		left: var(--space-3);
		right: var(--space-3);
		bottom: -2px;
		height: 2px;
		border-radius: 2px;
		background: var(--gradient-brand);
	}

	.actions {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-left: auto;
	}

	.search {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		height: 2.25rem;
		width: 15rem;
		padding-inline: var(--space-3) var(--space-2);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-full);
		background: var(--color-surface);
		font-size: var(--text-sm);
		color: var(--color-text-subtle);
		transition:
			border-color var(--duration-fast),
			color var(--duration-fast);
	}

	.search:hover {
		border-color: var(--color-border-strong);
		color: var(--color-text-muted);
	}

	.search kbd {
		margin-left: auto;
		font-size: 0.7rem;
		height: 1.5rem;
		padding-inline: 0.45rem;
	}

	.icon-link {
		display: grid;
		place-items: center;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: var(--radius-full);
		color: var(--color-text-muted);
		transition:
			color var(--duration-fast),
			background-color var(--duration-fast);
	}

	.icon-link:hover {
		color: var(--color-text);
		background: var(--color-surface-hover);
	}

	@media (max-width: 860px) {
		.search {
			width: 2.25rem;
			padding: 0;
			justify-content: center;
		}

		.search-label,
		.search kbd {
			display: none;
		}
	}

	@media (max-width: 560px) {
		nav {
			display: none;
		}
	}
</style>
