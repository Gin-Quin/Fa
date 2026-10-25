<script lang="ts">
	import type { ComponentProps } from "svelte";

	import GithubIcon from "#lib/components/GithubIcon.svelte";
	import Icon from "#lib/components/Icon.svelte";

	import CodeWindow from "./CodeWindow.svelte";

	let { files }: { files: ComponentProps<typeof CodeWindow>["files"] } = $props();

	// the floating squares of the logo, at fixed positions so that the server and the browser agree
	const PARTICLES = [
		{ x: 8, y: 22, size: 7, delay: 0 },
		{ x: 16, y: 58, size: 5, delay: 1.5 },
		{ x: 4, y: 78, size: 6, delay: 3 },
		{ x: 24, y: 12, size: 4, delay: 2.2 },
		{ x: 88, y: 18, size: 6, delay: 0.8 },
		{ x: 94, y: 46, size: 5, delay: 2.6 },
		{ x: 82, y: 70, size: 7, delay: 1.2 },
		{ x: 72, y: 8, size: 4, delay: 3.4 },
		{ x: 60, y: 30, size: 3, delay: 4 },
		{ x: 36, y: 34, size: 3, delay: 0.4 },
	];
</script>

<section class="hero">
	<div class="backdrop" aria-hidden="true">
		<div class="glow glow-orange"></div>
		<div class="glow glow-pink"></div>
		<div class="glow glow-purple"></div>
		<div class="grid"></div>
		{#each PARTICLES as particle, index (index)}
			<span
				class="particle"
				style:left="{particle.x}%"
				style:top="{particle.y}%"
				style:width="{particle.size}px"
				style:height="{particle.size}px"
				style:animation-delay="{particle.delay}s"
			></span>
		{/each}
	</div>

	<div class="container content">
		<a class="badge" href="/getting-started/introduction">
			<span class="badge-tag">In development</span>
			Read the language specification
			<Icon name="arrowRight" size={14} />
		</a>

		<h1>
			If it compiles,<br />
			<span class="gradient-text">it works.</span>
		</h1>

		<p class="lead">
			Fa is a programming language for <strong>AI agents</strong> and <strong>humans</strong>. Every
			function says what it returns, how it can fail, what it needs and what it changes, and the
			compiler holds it to its word.
		</p>

		<div class="actions">
			<a class="button button-primary button-large" href="/getting-started/introduction">
				Get started <Icon name="arrowRight" size={16} />
			</a>
			<a
				class="button button-secondary button-large"
				href="https://github.com/Gin-Quin/Fa"
				target="_blank"
				rel="noopener noreferrer"
			>
				<GithubIcon size={16} /> Star on GitHub
			</a>
		</div>

		<ul class="traits">
			<li>Between TypeScript and Rust</li>
			<li>No garbage collector</li>
			<li>No borrow checker</li>
			<li>Web, native &amp; WebAssembly</li>
		</ul>

		<div class="stage">
			<div class="window-glow" aria-hidden="true"></div>
			<div class="tilt">
				<CodeWindow {files} />
			</div>
		</div>
	</div>
</section>

<style>
	.hero {
		position: relative;
		margin-top: calc(-1 * var(--height-header));
		padding-top: calc(var(--height-header) + var(--space-9));
		padding-bottom: var(--space-9);
		overflow: hidden;
		isolation: isolate;
	}

	/* ─── Background ───────────────────────────────────── */

	.backdrop {
		position: absolute;
		inset: 0;
		z-index: -1;
		pointer-events: none;
	}

	.glow {
		position: absolute;
		border-radius: 50%;
		filter: blur(90px);
		animation: drift 18s var(--ease-in-out) infinite alternate;
	}

	.glow-orange {
		top: -12%;
		left: 8%;
		width: 42vw;
		height: 32vw;
		background: var(--glow-orange);
	}

	.glow-pink {
		top: 4%;
		left: 34%;
		width: 36vw;
		height: 30vw;
		background: var(--glow-pink);
		animation-delay: -6s;
	}

	.glow-purple {
		top: -8%;
		right: 4%;
		width: 40vw;
		height: 34vw;
		background: var(--glow-purple);
		animation-delay: -12s;
	}

	/* a floor of lines in perspective, like the grid of the logo */
	.grid {
		position: absolute;
		inset: 0;
		background-image:
			linear-gradient(var(--color-grid) 1px, transparent 1px),
			linear-gradient(90deg, var(--color-grid) 1px, transparent 1px);
		background-size: 64px 64px;
		background-position: center top;
		mask-image: radial-gradient(ellipse 70% 55% at 50% 30%, black 30%, transparent 75%);
	}

	.particle {
		position: absolute;
		border-radius: 2px;
		background: var(--gradient-brand);
		opacity: 0.7;
		animation: float 9s var(--ease-in-out) infinite;
	}

	/* ─── Content ──────────────────────────────────────── */

	.content {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
	}

	.badge {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: 0.3rem 0.85rem 0.3rem 0.3rem;
		border: 1px solid var(--color-border-strong);
		border-radius: var(--radius-full);
		background: color-mix(in srgb, var(--color-surface) 70%, transparent);
		backdrop-filter: blur(8px);
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		transition:
			border-color var(--duration-normal),
			color var(--duration-normal);
		animation: rise 0.9s var(--ease-out) both;
	}

	.badge:hover {
		border-color: color-mix(in srgb, var(--color-accent) 60%, transparent);
		color: var(--color-text);
	}

	.badge-tag {
		padding: 0.1rem 0.55rem;
		border-radius: var(--radius-full);
		background: var(--gradient-brand);
		font-size: var(--text-xs);
		font-weight: var(--weight-semibold);
		color: #fff;
	}

	h1 {
		margin-top: var(--space-6);
		font-family: var(--font-display);
		font-size: var(--text-hero);
		font-weight: var(--weight-bold);
		letter-spacing: -0.045em;
		line-height: 0.98;
		color: var(--color-text);
		animation: rise 0.9s var(--ease-out) 0.08s both;
	}

	h1 .gradient-text {
		padding-bottom: 0.08em;
		background-size: 200% 100%;
		animation: shimmer 8s linear infinite;
	}

	.lead {
		max-width: 40rem;
		margin-top: var(--space-6);
		font-size: var(--text-lg);
		line-height: 1.6;
		color: var(--color-text-muted);
		text-wrap: pretty;
		animation: rise 0.9s var(--ease-out) 0.16s both;
	}

	.lead strong {
		color: var(--color-text);
		font-weight: var(--weight-semibold);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: var(--space-3);
		margin-top: var(--space-6);
		animation: rise 0.9s var(--ease-out) 0.24s both;
	}

	.traits {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: var(--space-2) var(--space-5);
		margin-top: var(--space-6);
		padding: 0;
		list-style: none;
		font-size: var(--text-sm);
		color: var(--color-text-subtle);
		animation: rise 0.9s var(--ease-out) 0.32s both;
	}

	.traits li {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
	}

	.traits li::before {
		content: "";
		width: 5px;
		height: 5px;
		border-radius: 1px;
		background: var(--gradient-brand);
		transform: rotate(45deg);
	}

	/* ─── Code window ──────────────────────────────────── */

	.stage {
		position: relative;
		width: 100%;
		max-width: 62rem;
		margin-top: var(--space-9);
		perspective: 1800px;
		animation: rise 1.1s var(--ease-out) 0.4s both;
	}

	.window-glow {
		position: absolute;
		inset: 8% 4% -4%;
		border-radius: var(--radius-xl);
		background: var(--gradient-brand);
		filter: blur(70px);
		opacity: 0.32;
	}

	.tilt {
		transform-origin: 50% 100%;
	}

	/* the window straightens up as it scrolls into view */
	@supports (animation-timeline: view()) {
		@media (prefers-reduced-motion: no-preference) {
			.tilt {
				animation: straighten linear both;
				animation-timeline: view();
				animation-range: entry 0% cover 45%;
			}
		}
	}

	@keyframes straighten {
		from {
			transform: rotateX(18deg) scale(0.94);
		}
		to {
			transform: none;
		}
	}

	@keyframes rise {
		from {
			opacity: 0;
			transform: translateY(18px);
		}
	}

	@keyframes shimmer {
		to {
			background-position: -200% 0;
		}
	}

	@keyframes drift {
		to {
			transform: translate(4vw, 3vw) scale(1.12);
		}
	}

	@keyframes float {
		0%,
		100% {
			transform: translateY(0) rotate(0deg);
			opacity: 0.7;
		}
		50% {
			transform: translateY(-18px) rotate(45deg);
			opacity: 0.25;
		}
	}

	@media (max-width: 520px) {
		.badge {
			padding-left: 0.85rem;
			white-space: nowrap;
		}

		.badge-tag {
			display: none;
		}
	}

	@media (max-width: 720px) {
		.hero {
			padding-top: calc(var(--height-header) + var(--space-7));
		}

		.stage {
			margin-top: var(--space-7);
		}
	}
</style>
