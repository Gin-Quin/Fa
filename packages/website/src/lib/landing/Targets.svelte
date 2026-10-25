<script lang="ts">
	import Icon, { type IconName } from "#lib/components/Icon.svelte";

	import { reveal } from "./reveal.ts";
	import SectionHeader from "./SectionHeader.svelte";

	const TARGETS: Array<{ name: string; detail: string; angle: number }> = [
		{ name: "Browser", detail: "JavaScript", angle: -90 },
		{ name: "Bun", detail: "the default", angle: -38.6 },
		{ name: "Node.js", detail: "JavaScript", angle: 12.9 },
		{ name: "Deno", detail: "JavaScript", angle: 64.3 },
		{ name: "Native", detail: "machine code", angle: 115.7 },
		{ name: "WebAssembly", detail: "anywhere", angle: 167.1 },
		{ name: "Rust", detail: "source code", angle: 218.6 },
	];

	const USES: Array<{ icon: IconName; title: string; text: string }> = [
		{ icon: "terminal", title: "Tooling", text: "CLIs, build tools, servers and scripts." },
		{
			icon: "globe",
			title: "Applications",
			text: "Web, desktop and mobile, with built-in reactivity.",
		},
		{ icon: "box", title: "Games", text: "Native speed, columnar data, no GC pauses." },
	];

	// positions on a circle, in percent of the hub
	const position = (angle: number) => ({
		x: 50 + 41 * Math.cos((angle * Math.PI) / 180),
		y: 50 + 41 * Math.sin((angle * Math.PI) / 180),
	});
</script>

<section class="landing-section" id="targets">
	<div class="container">
		<SectionHeader eyebrow="One language, every platform">
			{#snippet title()}
				Write it once. <span class="gradient-text">Run it everywhere.</span>
			{/snippet}
			The same Fa code compiles to JavaScript for the browser and the server, to native code, to WebAssembly,
			or to Rust source code.
		</SectionHeader>

		<div class="layout">
			<div class="hub" {@attach reveal()}>
				<svg class="rays" viewBox="0 0 100 100" aria-hidden="true">
					{#each TARGETS as target, index (target.name)}
						{@const point = position(target.angle)}
						<line
							x1="50"
							y1="50"
							x2={point.x}
							y2={point.y}
							style:animation-delay="{index * 0.35}s"
						/>
					{/each}
					<circle cx="50" cy="50" r="41" />
					<circle cx="50" cy="50" r="27" />
				</svg>

				<div class="core">
					<img src="/fa_icon_128x128.webp" alt="Fa" width="64" height="64" />
				</div>

				{#each TARGETS as target (target.name)}
					{@const point = position(target.angle)}
					<div class="target" style:left="{point.x}%" style:top="{point.y}%">
						<span class="target-name">{target.name}</span>
						<span class="target-detail">{target.detail}</span>
					</div>
				{/each}
			</div>

			<div class="uses">
				{#each USES as use, index (use.title)}
					<div class="use" {@attach reveal(index * 100)}>
						<span class="icon"><Icon name={use.icon} size={20} /></span>
						<div>
							<h3>{use.title}</h3>
							<p>{use.text}</p>
						</div>
					</div>
				{/each}
				<a class="more" href="/architecture/packages#targets">
					Targets <Icon name="arrowRight" size={14} />
				</a>
			</div>
		</div>
	</div>
</section>

<style>
	.layout {
		display: grid;
		grid-template-columns: minmax(0, 1.2fr) minmax(0, 0.8fr);
		align-items: center;
		gap: var(--space-8);
	}

	.hub {
		position: relative;
		width: min(100%, 34rem);
		aspect-ratio: 1;
		margin-inline: auto;
	}

	.rays {
		position: absolute;
		inset: 0;
		overflow: visible;
	}

	.rays line {
		stroke: var(--brand-pink);
		stroke-width: 0.35;
		stroke-dasharray: 1.2 1.6;
		opacity: 0.55;
		animation: flow 2.4s linear infinite;
	}

	.rays circle {
		fill: none;
		stroke: var(--color-border);
		stroke-width: 0.25;
	}

	.core {
		position: absolute;
		left: 50%;
		top: 50%;
		display: grid;
		place-items: center;
		width: 7.5rem;
		height: 7.5rem;
		border: 1.5px solid transparent;
		border-radius: 2rem;
		background:
			linear-gradient(var(--color-surface), var(--color-surface)) padding-box,
			var(--gradient-brand) border-box;
		box-shadow:
			0 0 80px var(--glow-pink),
			0 0 0 10px color-mix(in srgb, var(--color-surface) 50%, transparent);
		transform: translate(-50%, -50%);
	}

	.target {
		position: absolute;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: var(--space-2) var(--space-4);
		border: 1px solid var(--color-border-strong);
		border-radius: var(--radius-md);
		background: var(--color-background-elevated);
		box-shadow: var(--shadow-md);
		transform: translate(-50%, -50%);
		white-space: nowrap;
		transition:
			border-color var(--duration-normal),
			transform var(--duration-normal) var(--ease-out);
	}

	.target:hover {
		border-color: color-mix(in srgb, var(--brand-pink) 50%, transparent);
		transform: translate(-50%, -50%) scale(1.06);
	}

	.target-name {
		font-size: var(--text-sm);
		font-weight: var(--weight-semibold);
		color: var(--color-text);
	}

	.target-detail {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--color-text-subtle);
	}

	.uses {
		display: grid;
		gap: var(--space-4);
	}

	.use {
		display: flex;
		gap: var(--space-4);
		padding: var(--space-5);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
	}

	.icon {
		display: grid;
		place-items: center;
		flex-shrink: 0;
		width: 2.75rem;
		height: 2.75rem;
		border-radius: var(--radius-md);
		color: #fff;
		background: var(--gradient-brand);
		box-shadow: var(--shadow-glow);
	}

	h3 {
		font-size: var(--text-md);
		font-weight: var(--weight-semibold);
		color: var(--color-text);
	}

	.use p {
		margin-top: var(--space-1);
		font-size: var(--text-sm);
		color: var(--color-text-muted);
	}

	.more {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		justify-self: start;
		font-size: var(--text-sm);
		font-weight: var(--weight-semibold);
		color: var(--color-accent-text);
	}

	@keyframes flow {
		to {
			stroke-dashoffset: -5.6;
		}
	}

	@media (max-width: 900px) {
		.layout {
			grid-template-columns: minmax(0, 1fr);
		}
	}

	@media (max-width: 520px) {
		.target {
			padding: var(--space-1) var(--space-2);
		}

		.target-detail {
			display: none;
		}

		.core {
			width: 5rem;
			height: 5rem;
			border-radius: 1.4rem;
		}

		.core img {
			width: 44px;
			height: 44px;
		}
	}
</style>
