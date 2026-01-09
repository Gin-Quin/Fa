<script lang="ts">
	import type { Snippet } from "svelte";

	type Props = {
		href: string;
		children?: Snippet<[]>;
	};

	const { href, children }: Props = $props();
</script>

<a {href}>
	<button>
		{@render children?.()}
	</button>
</a>

<style>
	button {
		padding: 1rem 2rem;
		cursor: pointer;
		color: #000;
		font-size: 1.2rem;
		font-weight: bold;
		border: none;
		position: relative;
		background: white;
	}

	button::before,
	button::after {
		--last-color: transparent;
		content: "";
		position: absolute;
		inset: -2px;
		background-image: conic-gradient(
			from var(--angle),
			#ed8600ff,
			#d900d5ff 50%,
			#ed8600ff 70%,
			var(--last-color) 80%
		);
		z-index: -1;
		animation: spin 5s linear infinite;
		transition: 110ms;
	}

	button::before {
		filter: blur(12px);
		opacity: 0.5;
	}

	@property --angle {
		syntax: "<angle>";
		initial-value: 0deg;
		inherits: false;
	}

	@keyframes spin {
		from {
			--angle: 0deg;
		}
		to {
			--angle: 360deg;
		}
	}

	button:hover {
		&::before,
		&::after {
			--last-color: #ed8600ff;
			transform: scale(1.1, 1.15) rotate(7deg);
			/*animation-play-state: paused;*/
		}
	}
</style>
