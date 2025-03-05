<script lang="ts">
	import type { Snippet } from "svelte"

	type Props = {
		href: string
		children?: Snippet<[]>
	}

	const { href, children }: Props = $props()
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
		border-radius: 1rem;
		border: none;
		position: relative;
		background: white;
		transition: 0.1s;
	}

	button::after {
		content: "";
		width: 100%;
		height: 100%;
		background-image: radial-gradient(
			circle farthest-corner at 10% 20%,
			#ed8600ff 17.8%,
			#d900d5ff 100.2%
		);
		filter: blur(15px);
		z-index: -1;
		position: absolute;
		left: 0;
		top: 0;
	}

	button:active {
		transform: scale(0.9) rotate(3deg);
		color: #fff;
		background: radial-gradient(
			circle farthest-corner at 10% 20%,
			#ed8600ff 17.8%,
			#d900d5ff 100.2%
		);
		transition:
			all 0.5s,
			color 0.3s;
	}
</style>
