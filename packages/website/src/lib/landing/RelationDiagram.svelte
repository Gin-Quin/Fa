<!--
	An animated relation: Ana's `bestFriend` is Bao, so Bao's `bestFriendOf` contains Ana.
	When Bao is deleted, both sides are cleaned up: `ana.bestFriend` becomes `none`.
-->
<figure class="diagram">
	<svg viewBox="0 0 520 300" role="img" aria-labelledby="relation-title">
		<title id="relation-title">
			Ana's best friend is Bao. When Bao is deleted from people, Ana's best friend becomes none.
		</title>
		<defs>
			<linearGradient id="relation-gradient" x1="0" x2="1" y1="0" y2="0">
				<stop offset="0" stop-color="var(--brand-orange)" />
				<stop offset="0.5" stop-color="var(--brand-pink)" />
				<stop offset="1" stop-color="var(--brand-purple)" />
			</linearGradient>
			<marker
				id="relation-arrow"
				viewBox="0 0 10 10"
				refX="8"
				refY="5"
				markerWidth="7"
				markerHeight="7"
				orient="auto-start-reverse"
			>
				<path d="M0 0 10 5 0 10z" fill="var(--brand-pink)" />
			</marker>
		</defs>

		<rect class="bag" x="10" y="10" width="500" height="280" rx="18" />
		<text class="bag-label" x="32" y="42">people: Bag(Person)</text>
		<text class="event" x="488" y="42" text-anchor="end">people.delete(bao)</text>

		<g class="links">
			<path class="link" d="M200 138 C 240 96, 280 96, 320 138" marker-end="url(#relation-arrow)" />
			<text class="link-label" x="260" y="96" text-anchor="middle">bestFriend</text>
			<path
				class="link reverse"
				d="M320 196 C 280 238, 240 238, 200 196"
				marker-end="url(#relation-arrow)"
			/>
			<text class="link-label" x="260" y="252" text-anchor="middle">bestFriendOf</text>
		</g>

		<g class="node">
			<rect x="30" y="118" width="170" height="96" rx="14" />
			<text class="name" x="50" y="152">Ana</text>
			<text class="field before" x="50" y="184">bestFriend: Bao</text>
			<text class="field after" x="50" y="184">bestFriend: none</text>
		</g>

		<g class="node deleted">
			<rect x="320" y="118" width="170" height="96" rx="14" />
			<text class="name" x="340" y="152">Bao</text>
			<text class="field" x="340" y="184">bestFriendOf: [Ana]</text>
		</g>
	</svg>
</figure>

<style>
	.diagram {
		--cycle: 10s;
		position: relative;
		padding: var(--space-5);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background:
			radial-gradient(circle at 30% 20%, var(--glow-pink), transparent 60%), var(--color-surface);
	}

	svg {
		width: 100%;
		height: auto;
		overflow: visible;
		font-family: var(--font-mono);
	}

	.bag {
		fill: none;
		stroke: var(--color-border-strong);
		stroke-dasharray: 6 6;
	}

	.bag-label {
		font-size: 13px;
		fill: var(--color-text-subtle);
	}

	.event {
		font-size: 13px;
		font-weight: 600;
		fill: var(--color-error);
		animation: event var(--cycle) infinite;
	}

	.link {
		fill: none;
		stroke: url(#relation-gradient);
		stroke-width: 2.5;
		stroke-dasharray: 200;
		animation: link var(--cycle) var(--ease-out) infinite;
	}

	.link.reverse {
		stroke-dasharray: 6 6;
		animation-name: link-reverse;
	}

	.link-label {
		font-size: 12px;
		fill: var(--color-text-muted);
		animation: fade-out var(--cycle) infinite;
	}

	.node rect {
		fill: var(--color-surface-raised);
		stroke: var(--color-border-strong);
		stroke-width: 1.5;
	}

	.name {
		font-family: var(--font-display);
		font-size: 18px;
		font-weight: 700;
		fill: var(--color-text);
	}

	.field {
		font-size: 12.5px;
		fill: var(--code-function);
	}

	.field.before {
		animation: fade-out var(--cycle) infinite;
	}

	.field.after {
		fill: var(--effect-mutates);
		opacity: 0;
		animation: fade-in var(--cycle) infinite;
	}

	.deleted {
		transform-box: fill-box;
		transform-origin: center;
		animation: deleted var(--cycle) infinite;
	}

	.deleted rect {
		animation: deleted-border var(--cycle) infinite;
	}

	@keyframes link {
		0% {
			stroke-dashoffset: 200;
			opacity: 1;
		}
		15%,
		46% {
			stroke-dashoffset: 0;
			opacity: 1;
		}
		54%,
		100% {
			stroke-dashoffset: 0;
			opacity: 0;
		}
	}

	@keyframes link-reverse {
		0%,
		10% {
			opacity: 0;
		}
		20%,
		46% {
			opacity: 1;
		}
		54%,
		100% {
			opacity: 0;
		}
	}

	@keyframes fade-out {
		0%,
		48% {
			opacity: 1;
		}
		54%,
		94% {
			opacity: 0;
		}
		100% {
			opacity: 1;
		}
	}

	@keyframes fade-in {
		0%,
		50% {
			opacity: 0;
		}
		56%,
		94% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}

	@keyframes event {
		0%,
		38% {
			opacity: 0;
		}
		42%,
		94% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}

	@keyframes deleted {
		0%,
		40% {
			opacity: 1;
			transform: none;
		}
		43% {
			transform: translateX(-4px);
		}
		46% {
			transform: translateX(4px);
		}
		49% {
			opacity: 1;
			transform: none;
		}
		56%,
		94% {
			opacity: 0.12;
			transform: scale(0.96);
		}
		100% {
			opacity: 1;
			transform: none;
		}
	}

	@keyframes deleted-border {
		0%,
		40% {
			stroke: var(--color-border-strong);
		}
		43%,
		94% {
			stroke: var(--color-error);
		}
		100% {
			stroke: var(--color-border-strong);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.diagram * {
			animation: none !important;
		}
	}
</style>
