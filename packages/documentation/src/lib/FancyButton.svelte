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
		color: var(--black);
		font-size: 1.2rem;
		font-weight: bold;
		border: none;
		position: relative;
		background: var(--white);
	}

	button::before,
	button::after {
		--transparent: var(--orangeDark);
		transform: scale(1.1, 1.15) rotate(7deg);
		content: "";
		position: absolute;
		inset: -2px;
		background-image: conic-gradient(
			from var(--angle),
			var(--transparent),
			var(--orangeDark) 5%,
			var(--purpleLight) 50%,
			var(--orangeDark) 75%,
			var(--transparent) 80%
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
			transform: scale(1) rotate(0);
			--transparent: transparent;
		}
	}
</style>
