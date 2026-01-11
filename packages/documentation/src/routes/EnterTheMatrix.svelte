<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { colors } from "../colors";

	type Props = {
		dotSize?: number;
		dotRadius?: number;
		rows?: number;
		dotsPerRow?: number;
		colorSet?: string[];
	};

	type Dot = {
		x: number;
		y: number;
		speed: number;
		opacity: number;
		color: string;
	};

	// Props using runes
	const {
		dotSize = 6,
		dotRadius = 0,
		rows = 15,
		dotsPerRow = 4,
		colorSet = Object.entries(colors)
			.filter(([key]) => !key.includes("Gray") && !key.includes("gray"))
			.map(([_, value]) => value),
	}: Props = $props();

	// Types for our component

	let container: HTMLDivElement;
	let dots = $state<Dot[]>([]);
	let animationFrameId: number;

	// Create dots when component mounts
	onMount(() => {
		// Create initial dots
		createDots();

		// Start animation
		animate();
	});

	onDestroy(() => {
		// Clean up
		if (animationFrameId) {
			cancelAnimationFrame(animationFrameId);
		}
	});

	function createDots() {
		const newDots: Dot[] = [];

		// Create dots for each row
		for (let row = 0; row < rows; row++) {
			for (let i = 0; i < dotsPerRow; i++) {
				newDots.push({
					x: Math.random() * container.clientWidth, // Start off-screen to the left
					y: row * dotSize,
					speed: Math.random() * 3 + 2, // Random speed between 2-7
					opacity: Math.random() * 0.5 + 0.5, // Random opacity between 0.5-1
					color: colorSet[Math.floor(Math.random() * colorSet.length)],
				});
			}
		}

		dots = newDots;
	}

	function animate() {
		for (const dot of dots) {
			dot.x += dot.speed;

			// Reset dot when it goes off-screen
			if (dot.x > container.clientWidth) {
				dot.x = Math.random() * -300; // Random position off-screen to the left
				dot.speed = Math.random() * 5 + 2; // New random speed
				dot.opacity = Math.random() * 0.5 + 0.5; // New random opacity
				dot.color = colorSet[Math.floor(Math.random() * colorSet.length)];
			}
		}

		// Continue animation
		animationFrameId = requestAnimationFrame(animate);
	}
</script>

<div
	class="matrix-container"
	bind:this={container}
	style="height: {rows * dotSize}px;"
>
	{#each dots as dot}
		<div
			class="dot"
			style:width={`${dotSize}px`}
			style:height={`${dotSize}px`}
			style:borderRadius={`${dotRadius}px`}
			style:left={`${dot.x}px`}
			style:top={`${dot.y}px`}
			style:opacity={1}
			style:background-color={`${dot.color}`}
		></div>
	{/each}
</div>

<style>
	.matrix-container {
		position: relative;
		width: 100%;
		overflow: hidden;
	}

	.dot {
		position: absolute;
		/* background-color: #0f0; Matrix green */
		/* box-shadow: 0 0 8px rgba(0, 255, 0, 0.7); */
		transform: translateZ(0); /* Hardware acceleration */
	}
</style>
