<script lang="ts">
	import "#lib/styles/prose.css";

	import Icon from "#lib/components/Icon.svelte";
	import TableOfContents from "#lib/docs/TableOfContents.svelte";

	import type { PageData } from "./$types";

	let { data }: { data: PageData } = $props();

	const page = $derived(data.page);

	/** The copy buttons of the code blocks are rendered from Markdown: they are handled here. */
	async function onclick(event: MouseEvent) {
		const button = (event.target as HTMLElement).closest<HTMLButtonElement>(".code-block-copy");
		const code = button?.closest(".code-block")?.querySelector("pre")?.textContent;
		if (!button || code == null) {
			return;
		}
		try {
			await navigator.clipboard.writeText(code);
			button.textContent = "Copied";
			button.dataset.copied = "";
		} catch {
			button.textContent = "Failed";
		}
		setTimeout(() => {
			button.textContent = "Copy";
			delete button.dataset.copied;
		}, 1600);
	}
</script>

<svelte:head>
	<title>{page.title} · Fa</title>
	<meta name="description" content={page.description} />
	<meta property="og:title" content="{page.title} · Fa" />
	<meta property="og:description" content={page.description} />
</svelte:head>

<div class="page">
	<article>
		<header>
			<p class="section">{page.section}</p>
			<h1>{page.title}</h1>
		</header>

		<!-- the HTML is rendered at build time from the Markdown files of the repository -->
		<div class="prose" role="presentation" {onclick}>{@html page.html}</div>

		<footer>
			<a class="edit" href={page.editUrl} target="_blank" rel="noopener noreferrer">
				<Icon name="edit" size={16} /> Edit this page on GitHub
			</a>

			<nav class="neighbours" aria-label="Previous and next pages">
				{#if data.previous}
					<a class="neighbour previous" href="/{data.previous.slug}">
						<span class="direction"><Icon name="arrowLeft" size={14} /> Previous</span>
						<span class="title">{data.previous.title}</span>
					</a>
				{/if}
				{#if data.next}
					<a class="neighbour next" href="/{data.next.slug}">
						<span class="direction">Next <Icon name="arrowRight" size={14} /></span>
						<span class="title">{data.next.title}</span>
					</a>
				{/if}
			</nav>
		</footer>
	</article>

	<aside class="toc">
		<TableOfContents entries={page.toc} />
	</aside>
</div>

<style>
	.page {
		display: grid;
		grid-template-columns: minmax(0, var(--width-prose)) var(--width-toc);
		justify-content: center;
		gap: var(--space-8);
		padding: var(--space-7) var(--gutter) var(--space-9);
	}

	header {
		margin-bottom: var(--space-6);
	}

	.section {
		margin-bottom: var(--space-2);
		font-size: var(--text-sm);
		font-weight: var(--weight-semibold);
	}

	.section {
		width: fit-content;
		background: var(--gradient-brand);
		-webkit-background-clip: text;
		background-clip: text;
		color: transparent;
	}

	h1 {
		font-family: var(--font-display);
		font-size: var(--text-3xl);
		font-weight: var(--weight-bold);
		letter-spacing: var(--tracking-tight);
		line-height: 1.1;
		color: var(--color-text);
	}

	footer {
		margin-top: var(--space-9);
		display: grid;
		gap: var(--space-6);
	}

	.edit {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		width: fit-content;
		font-size: var(--text-sm);
		color: var(--color-text-subtle);
		transition: color var(--duration-fast);
	}

	.edit:hover {
		color: var(--color-text);
	}

	.neighbours {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-4);
	}

	.neighbour {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-4) var(--space-5);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		transition:
			border-color var(--duration-normal),
			background-color var(--duration-normal),
			transform var(--duration-normal) var(--ease-out);
	}

	.neighbour:hover {
		border-color: color-mix(in srgb, var(--color-accent) 50%, transparent);
		background: var(--color-accent-soft);
	}

	.next {
		grid-column: 2;
		text-align: right;
		align-items: flex-end;
	}

	.direction {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		font-size: var(--text-xs);
		color: var(--color-text-subtle);
	}

	.neighbour .title {
		font-weight: var(--weight-semibold);
		color: var(--color-text);
	}

	.toc {
		position: sticky;
		top: calc(var(--height-header) + var(--space-7));
		align-self: start;
		max-height: calc(100vh - var(--height-header) - var(--space-8));
		overflow-y: auto;
		scrollbar-width: thin;
	}

	@media (max-width: 1240px) {
		.page {
			grid-template-columns: minmax(0, var(--width-prose));
		}

		.toc {
			display: none;
		}
	}

	@media (max-width: 560px) {
		.neighbours {
			grid-template-columns: 1fr;
		}

		.next {
			grid-column: 1;
		}
	}
</style>
