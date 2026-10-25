<script lang="ts">
	import Icon from "#lib/components/Icon.svelte";

	import { reveal } from "./reveal.ts";
	import SectionHeader from "./SectionHeader.svelte";

	const LANGUAGES = [
		{
			name: "TypeScript",
			slug: "typescript",
			text: "The same ergonomics, with types you can trust at runtime.",
		},
		{
			name: "Rust",
			slug: "rust",
			text: "The same guarantees, without lifetimes or a borrow checker.",
		},
		{
			name: "Go",
			slug: "go",
			text: "The same simplicity, with typed errors and generics that read well.",
		},
		{
			name: "Python",
			slug: "python",
			text: "The same readability, with static types and native speed.",
		},
		{
			name: "C / C++",
			slug: "c",
			text: "The same control over memory, without undefined behavior.",
		},
		{ name: "Java", slug: "java", text: "Errors in signatures, without checked exceptions." },
		{
			name: "C#",
			slug: "csharp",
			text: "Value types and structural typing, without a garbage collector.",
		},
	];
</script>

<section class="landing-section" id="coming-from">
	<div class="container">
		<SectionHeader eyebrow="Coming from another language">
			{#snippet title()}
				You already know <span class="gradient-text">most of it</span>
			{/snippet}
			Fa sits between TypeScript and Rust. Each guide maps what you know to how Fa does it.
		</SectionHeader>

		<div class="grid">
			{#each LANGUAGES as language, index (language.slug)}
				<a
					class="card"
					href="/fa-for/fa-for-{language.slug}-developers"
					{@attach reveal((index % 4) * 70)}
				>
					<span class="name">Fa for {language.name}</span>
					<span class="text">{language.text}</span>
					<span class="arrow"><Icon name="arrowRight" size={16} /></span>
				</a>
			{/each}
			<a class="card reference" href="/reference/language-reference" {@attach reveal(210)}>
				<span class="name">The whole language</span>
				<span class="text">Every construct of Fa, on one page.</span>
				<span class="arrow"><Icon name="arrowRight" size={16} /></span>
			</a>
		</div>
	</div>
</section>

<style>
	.grid {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: var(--space-3);
	}

	.card {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		min-height: 9rem;
		padding: var(--space-5);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
		transition:
			border-color var(--duration-normal),
			background-color var(--duration-normal),
			transform var(--duration-normal) var(--ease-out);
	}

	.card:hover {
		border-color: color-mix(in srgb, var(--brand-pink) 40%, var(--color-border));
		transform: translateY(-3px);
	}

	.reference {
		background:
			linear-gradient(var(--color-surface), var(--color-surface)) padding-box,
			var(--gradient-brand) border-box;
		border-color: transparent;
	}

	.name {
		font-weight: var(--weight-semibold);
		color: var(--color-text);
	}

	.text {
		font-size: var(--text-sm);
		color: var(--color-text-muted);
	}

	.arrow {
		margin-top: auto;
		color: var(--color-text-subtle);
		transition:
			color var(--duration-normal),
			transform var(--duration-normal) var(--ease-out);
	}

	.card:hover .arrow {
		color: var(--color-accent-text);
		transform: translateX(4px);
	}

	@media (max-width: 960px) {
		.grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	@media (max-width: 520px) {
		.grid {
			grid-template-columns: minmax(0, 1fr);
		}

		.card {
			min-height: 0;
		}
	}
</style>
