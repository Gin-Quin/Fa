<script lang="ts">
	import Icon from "#lib/components/Icon.svelte";

	import InlineCode from "./InlineCode.svelte";
	import RelationDiagram from "./RelationDiagram.svelte";
	import { reveal } from "./reveal.ts";
	import SectionHeader from "./SectionHeader.svelte";

	let { invalidation, precise }: { invalidation: string; precise: string } = $props();

	const ZEROES = [
		{ label: "garbage collector pauses", text: "Values are freed at the end of their scope." },
		{ label: "lifetime annotations", text: "References name their collection instead." },
		{ label: "dangling references", text: "Invalidated references must be declared again." },
	];
</script>

<section class="landing-section memory" id="memory">
	<div class="container">
		<SectionHeader eyebrow="Memory, made simple">
			{#snippet title()}
				Memory safe. <span class="gradient-text">No ceremony.</span>
			{/snippet}
			No garbage collector, no borrow checker, no lifetimes. Values belong to a scope or to a collection,
			and the type system knows where each one lives.
		</SectionHeader>

		<div class="row">
			<div class="text" {@attach reveal()}>
				<h3>Where data lives is part of the type</h3>
				<p>
					A reference into a collection has a type like <code>Person in persons</code>. The compiler
					tracks every operation that can move or remove elements, and only those.
				</p>
				<ul>
					<li>
						<Icon name="check" size={16} />
						<span
							><InlineCode text="Writing an element is a mutation: references stay valid." /></span
						>
					</li>
					<li>
						<Icon name="check" size={16} />
						<span
							><InlineCode
								text="`push` may reallocate: it invalidates the references, which must be declared again."
							/></span
						>
					</li>
					<li>
						<Icon name="check" size={16} />
						<span
							><InlineCode
								text="Invalidations are precise: `pop` only invalidates the last element."
							/></span
						>
					</li>
				</ul>
				<a class="more" href="/advanced/owned-references">
					Owned references <Icon name="arrowRight" size={14} />
				</a>
			</div>

			<div class="panels" {@attach reveal(120)}>
				<div class="panel with-line-numbers">
					{@html invalidation}
					<div class="diagnostic">
						<p>
							<span class="kind-error">error:</span>
							<InlineCode text="`someone` was invalidated by `persons.push`" />
						</p>
						<p class="location">&nbsp;&nbsp;--> people.fa:6:13</p>
						<p>
							&nbsp;&nbsp;<span class="help">help:</span>
							<InlineCode text="declare it again with `use someone = persons[...]`" />
						</p>
					</div>
				</div>
				<div class="panel small">
					{@html precise}
				</div>
			</div>
		</div>

		<div class="row reverse">
			<div class="text" {@attach reveal()}>
				<h3>Relations instead of weak pointers</h3>
				<p>
					A field that points to another object names its collection, and declares its other side.
					The compiler keeps both sides in sync on every write, and cleans them up when an object is
					deleted.
				</p>
				<ul>
					<li>
						<Icon name="check" size={16} />
						<span>One-to-one, one-to-many and many-to-many, declared in the type.</span>
					</li>
					<li>
						<Icon name="check" size={16} />
						<span>Deleting an object costs one step per link: reading a link costs nothing.</span>
					</li>
					<li>
						<Icon name="check" size={16} />
						<span>No generation counters, no reference counting, nothing left dangling.</span>
					</li>
				</ul>
				<a class="more" href="/advanced/relations">
					Relations <Icon name="arrowRight" size={14} />
				</a>
			</div>

			<div {@attach reveal(120)}>
				<RelationDiagram />
			</div>
		</div>

		<div class="zeroes">
			{#each ZEROES as zero, index (zero.label)}
				<div class="zero" {@attach reveal(index * 100)}>
					<p class="number gradient-text">0</p>
					<p class="label">{zero.label}</p>
					<p class="detail">{zero.text}</p>
				</div>
			{/each}
		</div>
	</div>
</section>

<style>
	.row {
		display: grid;
		grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
		align-items: center;
		gap: var(--space-8);
	}

	.row + .row {
		margin-top: var(--space-9);
	}

	.row.reverse {
		grid-template-columns: minmax(0, 1.1fr) minmax(0, 0.9fr);
	}

	.row.reverse .text {
		order: 2;
	}

	h3 {
		font-size: var(--text-2xl);
		font-weight: var(--weight-bold);
		letter-spacing: var(--tracking-tight);
		line-height: 1.15;
		color: var(--color-text);
	}

	.text > p {
		margin-top: var(--space-4);
		font-size: var(--text-md);
		color: var(--color-text-muted);
	}

	.text :global(code) {
		padding: 0.1em 0.35em;
		border-radius: 5px;
		background: var(--code-inline-background);
		color: var(--code-inline-text);
		font-size: 0.88em;
	}

	ul {
		display: grid;
		gap: var(--space-3);
		margin-top: var(--space-5);
		padding: 0;
		list-style: none;
	}

	li {
		display: flex;
		gap: var(--space-3);
		font-size: var(--text-sm);
		color: var(--color-text-muted);
	}

	li > :global(svg) {
		flex-shrink: 0;
		margin-top: 0.2rem;
		color: var(--color-success);
	}

	.more {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		margin-top: var(--space-5);
		font-size: var(--text-sm);
		font-weight: var(--weight-semibold);
		color: var(--color-accent-text);
	}

	.more:hover {
		gap: var(--space-2);
	}

	.panels {
		display: grid;
		gap: var(--space-4);
	}

	.small {
		margin-left: var(--space-8);
	}

	.zeroes {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		margin-top: var(--space-9);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
		overflow: hidden;
	}

	.zero {
		padding: var(--space-6);
		text-align: center;
	}

	.zero + .zero {
		border-left: 1px solid var(--color-border);
	}

	.number {
		width: fit-content;
		margin-inline: auto;
		font-family: var(--font-display);
		font-size: var(--text-4xl);
		font-weight: var(--weight-bold);
		line-height: 1;
	}

	.label {
		margin-top: var(--space-3);
		font-weight: var(--weight-semibold);
		color: var(--color-text);
	}

	.detail {
		margin-top: var(--space-1);
		font-size: var(--text-sm);
		color: var(--color-text-muted);
	}

	@media (max-width: 900px) {
		.row,
		.row.reverse {
			grid-template-columns: minmax(0, 1fr);
			gap: var(--space-6);
		}

		.row.reverse .text {
			order: 0;
		}

		.small {
			margin-left: 0;
		}

		.zeroes {
			grid-template-columns: minmax(0, 1fr);
		}

		.zero + .zero {
			border-left: none;
			border-top: 1px solid var(--color-border);
		}
	}
</style>
