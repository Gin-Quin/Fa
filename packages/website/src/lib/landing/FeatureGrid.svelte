<script lang="ts">
	import Icon, { type IconName } from "#lib/components/Icon.svelte";

	import { reveal } from "./reveal.ts";
	import SectionHeader from "./SectionHeader.svelte";

	type FeatureName =
		| "errors"
		| "requirements"
		| "words"
		| "reactivity"
		| "parallel"
		| "compileTime"
		| "tests"
		| "integers";

	let { code }: { code: Record<FeatureName, string> } = $props();

	const FEATURES: Array<{
		name: FeatureName;
		icon: IconName;
		title: string;
		text: string;
		href: string;
		wide?: boolean;
	}> = [
		{
			name: "errors",
			icon: "shield",
			title: "Typed errors, no exceptions",
			text: "Errors are values declared with throws. Propagate them with try, handle them with catch: an error never escapes silently.",
			href: "/values/errors",
			wide: true,
		},
		{
			name: "requirements",
			icon: "layers",
			title: "Dependency injection, built in",
			text: "Provide a service once; every function down the call chain that requires it receives it.",
			href: "/advanced/requirements",
		},
		{
			name: "words",
			icon: "book",
			title: "Words over symbols",
			text: "No truthy values, no implicit conversions: is, not, and, or say exactly what they mean.",
			href: "/operations/logical",
		},
		{
			name: "reactivity",
			icon: "refresh",
			title: "Reactive values",
			text: "reactive, derived and watch replace signals, hooks and stores, on the frontend and the backend.",
			href: "/advanced/reactivity",
		},
		{
			name: "parallel",
			icon: "bolt",
			title: "Parallel by keyword",
			text: "Add parallel to a loop. The compiler proves that the iterations can't race.",
			href: "/advanced/concurrency",
		},
		{
			name: "compileTime",
			icon: "wand",
			title: "Code that runs at compile time",
			text: "Everything prefixed with @ runs while compiling: conditions, loops, types computed from values.",
			href: "/advanced/metaprogramming",
		},
		{
			name: "tests",
			icon: "test",
			title: "Tests are part of the language",
			text: "Write test blocks next to the code they check, and run them with the compiler.",
			href: "/architecture/testing",
		},
		{
			name: "integers",
			icon: "cpu",
			title: "Integers that never overflow silently",
			text: "Ranges are tracked at compile time: an impossible overflow costs nothing, a possible one is checked.",
			href: "/values/numbers",
		},
	];

	/** Moves the spotlight of a card under the pointer. */
	function onpointermove(event: PointerEvent) {
		const card = event.currentTarget as HTMLElement;
		const bounds = card.getBoundingClientRect();
		card.style.setProperty("--x", `${event.clientX - bounds.left}px`);
		card.style.setProperty("--y", `${event.clientY - bounds.top}px`);
	}
</script>

<section class="landing-section" id="features">
	<div class="container">
		<SectionHeader eyebrow="One way to write it">
			{#snippet title()}
				A small language that <span class="gradient-text">reads like prose</span>
			{/snippet}
			Fa favors one obvious construct over many clever ones. Code is easy to read for humans, and hard
			to get wrong for agents.
		</SectionHeader>

		<div class="grid">
			{#each FEATURES as feature, index (feature.name)}
				<a
					class="card"
					class:wide={feature.wide}
					href={feature.href}
					{onpointermove}
					{@attach reveal((index % 3) * 80)}
				>
					<div class="card-text">
						<span class="icon"><Icon name={feature.icon} size={18} /></span>
						<h3>{feature.title}</h3>
						<p>{feature.text}</p>
					</div>
					<div class="snippet">{@html code[feature.name]}</div>
				</a>
			{/each}
		</div>
	</div>
</section>

<style>
	.grid {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: var(--space-4);
	}

	.card {
		--x: 50%;
		--y: 0%;
		position: relative;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		gap: var(--space-5);
		padding: var(--space-5);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
		overflow: hidden;
		isolation: isolate;
		transition:
			border-color var(--duration-normal),
			transform var(--duration-normal) var(--ease-out);
	}

	/* the spotlight */
	.card::before {
		content: "";
		position: absolute;
		inset: 0;
		z-index: -1;
		background: radial-gradient(
			420px circle at var(--x) var(--y),
			color-mix(in srgb, var(--brand-pink) 14%, transparent),
			transparent 60%
		);
		opacity: 0;
		transition: opacity var(--duration-normal);
	}

	.card:hover {
		border-color: color-mix(in srgb, var(--brand-pink) 35%, var(--color-border));
	}

	.card:hover::before {
		opacity: 1;
	}

	.wide {
		grid-column: span 2;
		flex-direction: row;
		align-items: center;
	}

	.wide .card-text {
		flex: 1;
	}

	.wide .snippet {
		flex: 1.2;
	}

	.icon {
		display: grid;
		place-items: center;
		width: 2.25rem;
		height: 2.25rem;
		margin-bottom: var(--space-4);
		border: 1px solid var(--color-border-strong);
		border-radius: var(--radius-md);
		color: var(--color-accent-text);
		background: var(--color-accent-soft);
	}

	h3 {
		font-size: var(--text-md);
		font-weight: var(--weight-semibold);
		letter-spacing: var(--tracking-snug);
		color: var(--color-text);
	}

	p {
		margin-top: var(--space-2);
		font-size: var(--text-sm);
		line-height: 1.6;
		color: var(--color-text-muted);
	}

	.snippet {
		border: 1px solid var(--code-border);
		border-radius: var(--radius-md);
		background: var(--code-background);
		overflow: hidden;
	}

	:root[data-theme="light"] .snippet {
		border-color: transparent;
	}

	.snippet :global(pre.shiki) {
		padding: var(--space-3) var(--space-4);
		font-size: 0.75rem;
		line-height: 1.7;
	}

	@media (max-width: 960px) {
		.grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	@media (max-width: 640px) {
		.grid {
			grid-template-columns: minmax(0, 1fr);
		}

		.wide {
			grid-column: auto;
			flex-direction: column;
			align-items: stretch;
		}
	}
</style>
