<script lang="ts">
	import { goto } from "$app/navigation";
	import { tick } from "svelte";

	import Icon from "#lib/components/Icon.svelte";

	import { query, search, type SearchPage, type SearchResult } from "./search.svelte.ts";

	let input = $state("");
	let pages = $state<SearchPage[]>([]);
	let selected = $state(0);
	let field = $state<HTMLInputElement>();
	let list = $state<HTMLUListElement>();

	const results: SearchResult[] = $derived(query(pages, input));

	const SUGGESTIONS = [
		{ label: "Typed errors", href: "/values/errors" },
		{ label: "Owned references", href: "/advanced/owned-references" },
		{ label: "Relations", href: "/advanced/relations" },
		{ label: "Requirements", href: "/advanced/requirements" },
		{ label: "Language reference", href: "/reference/language-reference" },
	];

	$effect(() => {
		if (search.open) {
			search.load().then((loaded) => (pages = loaded));
			tick().then(() => field?.focus());
		}
	});

	$effect(() => {
		// the first result is selected after each new query
		input;
		selected = 0;
	});

	function onGlobalKeydown(event: KeyboardEvent) {
		const target = event.target as HTMLElement | null;
		const typing =
			target instanceof HTMLInputElement ||
			target instanceof HTMLTextAreaElement ||
			target?.isContentEditable;
		if ((event.key === "k" && (event.metaKey || event.ctrlKey)) || (event.key === "/" && !typing)) {
			event.preventDefault();
			search.open ? search.hide() : search.show();
		}
	}

	function open(href: string) {
		search.hide();
		input = "";
		goto(href);
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") {
			search.hide();
		} else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
			event.preventDefault();
			const count = results.length;
			if (count > 0) {
				selected = (selected + (event.key === "ArrowDown" ? 1 : count - 1)) % count;
				tick().then(() =>
					list?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" }),
				);
			}
		} else if (event.key === "Enter" && results[selected]) {
			event.preventDefault();
			open(results[selected].href);
		}
	}
</script>

<svelte:window onkeydown={onGlobalKeydown} />

{#if search.open}
	<div class="backdrop" role="presentation" onclick={() => search.hide()}></div>
	<div class="dialog" role="dialog" aria-modal="true" aria-label="Search the documentation">
		<div class="field">
			<Icon name="search" size={20} />
			<input
				bind:this={field}
				bind:value={input}
				onkeydown={onKeydown}
				type="search"
				placeholder="Search the documentation"
				autocomplete="off"
				spellcheck="false"
				role="combobox"
				aria-expanded={results.length > 0}
				aria-controls="search-results"
				aria-activedescendant={results.length > 0 ? `search-result-${selected}` : undefined}
			/>
			<kbd>Esc</kbd>
		</div>

		<div class="body">
			{#if input.trim() === ""}
				<p class="label">Popular pages</p>
				<ul class="suggestions">
					{#each SUGGESTIONS as suggestion (suggestion.href)}
						<li>
							<a href={suggestion.href} onclick={() => search.hide()}>
								<Icon name="file" size={16} />
								{suggestion.label}
							</a>
						</li>
					{/each}
				</ul>
			{:else if results.length === 0}
				<p class="empty">No results for “{input}”</p>
			{:else}
				<ul id="search-results" class="results" role="listbox" bind:this={list}>
					{#each results as result, index (result.href)}
						<li
							id="search-result-{index}"
							role="option"
							aria-selected={index === selected}
							onmousemove={() => (selected = index)}
						>
							<a
								href={result.href}
								onclick={(event) => {
									event.preventDefault();
									open(result.href);
								}}
							>
								<Icon name={result.heading ? "hash" : "file"} size={16} />
								<span class="result">
									<span class="path">
										{result.section} / {result.title}
										{#if result.heading}<span class="heading"> / {result.heading}</span>{/if}
									</span>
									<span class="snippet">
										{#each result.snippet as part, partIndex (partIndex)}
											{#if part.match}<mark>{part.text}</mark>{:else}{part.text}{/if}
										{/each}
									</span>
								</span>
								<Icon name="chevronRight" size={16} class="enter" />
							</a>
						</li>
					{/each}
				</ul>
			{/if}
		</div>

		<footer>
			<span><kbd>↑</kbd><kbd>↓</kbd> to navigate</span>
			<span><kbd>↵</kbd> to open</span>
		</footer>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: var(--layer-overlay);
		background: var(--color-overlay);
		backdrop-filter: blur(6px);
		animation: fade var(--duration-normal) var(--ease-out);
	}

	.dialog {
		position: fixed;
		top: min(12vh, 8rem);
		left: 50%;
		z-index: var(--layer-modal);
		display: flex;
		flex-direction: column;
		width: min(40rem, calc(100vw - 2 * var(--space-4)));
		max-height: min(36rem, 76vh);
		border: 1px solid var(--color-border-strong);
		border-radius: var(--radius-lg);
		background: var(--color-background-elevated);
		box-shadow: var(--shadow-lg);
		transform: translateX(-50%);
		overflow: hidden;
		animation: rise var(--duration-normal) var(--ease-out);
	}

	.field {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-4) var(--space-5);
		border-bottom: 1px solid var(--color-border);
		color: var(--color-text-subtle);
	}

	input {
		flex: 1;
		min-width: 0;
		border: none;
		outline: none;
		background: none;
		font-size: var(--text-md);
		color: var(--color-text);
	}

	input::placeholder {
		color: var(--color-text-subtle);
	}

	input::-webkit-search-cancel-button {
		display: none;
	}

	.body {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-2);
	}

	.label {
		padding: var(--space-2) var(--space-3);
		font-size: var(--text-xs);
		font-weight: var(--weight-semibold);
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
		color: var(--color-text-subtle);
	}

	ul {
		list-style: none;
		padding: 0;
	}

	.suggestions a {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3);
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
		font-size: var(--text-sm);
	}

	.suggestions a:hover {
		background: var(--color-surface-hover);
		color: var(--color-text);
	}

	.empty {
		padding: var(--space-7) var(--space-4);
		text-align: center;
		color: var(--color-text-subtle);
	}

	.results a {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		padding: var(--space-3);
		border-radius: var(--radius-md);
		color: var(--color-text-subtle);
	}

	.results a > :global(svg) {
		flex-shrink: 0;
		margin-top: 0.15rem;
	}

	.results [aria-selected="true"] a {
		background: var(--color-accent-soft);
		color: var(--color-accent-text);
	}

	.results a :global(.enter) {
		margin-left: auto;
		opacity: 0;
	}

	.results [aria-selected="true"] a :global(.enter) {
		opacity: 1;
	}

	.result {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 0;
	}

	.path {
		font-size: var(--text-sm);
		font-weight: var(--weight-medium);
		color: var(--color-text);
	}

	.heading {
		color: var(--color-text-muted);
	}

	.snippet {
		font-size: var(--text-sm);
		line-height: 1.5;
		color: var(--color-text-muted);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	mark {
		background: none;
		color: var(--color-accent-text);
		font-weight: var(--weight-semibold);
	}

	footer {
		display: flex;
		gap: var(--space-5);
		padding: var(--space-3) var(--space-5);
		border-top: 1px solid var(--color-border);
		font-size: var(--text-xs);
		color: var(--color-text-subtle);
	}

	footer span {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
	}

	@keyframes fade {
		from {
			opacity: 0;
		}
	}

	@keyframes rise {
		from {
			opacity: 0;
			transform: translate(-50%, 8px) scale(0.98);
		}
	}
</style>
