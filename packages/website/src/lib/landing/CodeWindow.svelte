<script lang="ts">
	import InlineCode from "./InlineCode.svelte";

	type File = {
		name: string;
		label: string;
		html: string;
		diagnostic: { kind: "error" | "success"; message: string; location: string; help: string };
	};

	let { files }: { files: File[] } = $props();

	const DURATION = 7000;

	let active = $state(0);
	let autoplay = $state(true);
	let paused = $state(false);

	// cycles through the files until the reader picks one
	$effect(() => {
		if (!autoplay || paused) {
			return;
		}
		const timer = setTimeout(() => (active = (active + 1) % files.length), DURATION);
		return () => clearTimeout(timer);
	});

	function select(index: number) {
		active = index;
		autoplay = false;
	}

	const file = $derived(files[active]);
</script>

<div
	class="window"
	role="region"
	aria-label="Examples of Fa code"
	onmouseenter={() => (paused = true)}
	onmouseleave={() => (paused = false)}
>
	<div class="chrome">
		<div class="lights" aria-hidden="true"><span></span><span></span><span></span></div>
		<div class="tabs" role="tablist">
			{#each files as item, index (item.name)}
				<button
					type="button"
					role="tab"
					class="tab"
					aria-selected={index === active}
					onclick={() => select(index)}
				>
					<span class="file-icon" aria-hidden="true"></span>
					{item.name}
					{#if index === active && autoplay}
						{#key active}
							<span
								class="progress"
								style:animation-duration="{DURATION}ms"
								style:animation-play-state={paused ? "paused" : "running"}
							></span>
						{/key}
					{/if}
				</button>
			{/each}
		</div>
		<span class="label">{file.label}</span>
	</div>

	<div class="editor with-line-numbers" role="tabpanel">
		{#key active}
			<div class="code">{@html file.html}</div>
		{/key}
	</div>

	<div class="diagnostic">
		{#key active}
			<div class="diagnostic-content">
				<p>
					<span class="prompt">$</span> fa check
				</p>
				<p>
					<span class="kind-{file.diagnostic.kind}"
						>{file.diagnostic.kind === "error" ? "error:" : "✓"}</span
					>
					<InlineCode text={file.diagnostic.message} />
				</p>
				<p class="location">&nbsp;&nbsp;--> {file.diagnostic.location}</p>
				<p>
					&nbsp;&nbsp;<span class="help"
						>{file.diagnostic.kind === "error" ? "help:" : "note:"}</span
					>
					<InlineCode text={file.diagnostic.help} />
				</p>
			</div>
		{/key}
	</div>
</div>

<style>
	.window {
		position: relative;
		border: 1px solid rgb(255 255 255 / 0.1);
		border-radius: var(--radius-xl);
		background: color-mix(in srgb, var(--code-background) 92%, transparent);
		box-shadow:
			0 50px 120px -30px rgb(0 0 0 / 0.75),
			0 0 0 1px rgb(0 0 0 / 0.3),
			inset 0 1px 0 rgb(255 255 255 / 0.06);
		backdrop-filter: blur(20px);
		overflow: hidden;
		text-align: left;
	}

	.chrome {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		height: 3rem;
		padding-inline: var(--space-4);
		border-bottom: 1px solid var(--code-border);
		background: var(--code-header);
	}

	.lights {
		display: flex;
		gap: 7px;
		flex-shrink: 0;
	}

	.lights span {
		width: 11px;
		height: 11px;
		border-radius: 50%;
		background: #2a2a38;
	}

	.lights span:nth-child(1) {
		background: #ff5f57;
	}

	.lights span:nth-child(2) {
		background: #febc2e;
	}

	.lights span:nth-child(3) {
		background: #28c840;
	}

	.tabs {
		display: flex;
		align-self: stretch;
		gap: 2px;
		overflow-x: auto;
		scrollbar-width: none;
	}

	.tab {
		position: relative;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding-inline: var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--code-muted);
		white-space: nowrap;
		transition:
			color var(--duration-fast),
			background-color var(--duration-fast);
	}

	.tab:hover {
		color: var(--code-text);
	}

	.tab[aria-selected="true"] {
		color: var(--code-text);
		background: var(--code-background);
	}

	.tab[aria-selected="true"]::after {
		content: "";
		position: absolute;
		inset: 0 0 auto;
		height: 2px;
		background: var(--gradient-brand);
	}

	.file-icon {
		width: 8px;
		height: 8px;
		border-radius: 2px;
		background: var(--gradient-brand);
		transform: rotate(45deg);
		opacity: 0.5;
	}

	.tab[aria-selected="true"] .file-icon {
		opacity: 1;
	}

	.progress {
		position: absolute;
		left: 0;
		bottom: 0;
		height: 1px;
		width: 100%;
		background: rgb(255 255 255 / 0.35);
		transform-origin: left;
		animation: progress linear forwards;
	}

	.label {
		margin-left: auto;
		padding: 0.2rem 0.65rem;
		border: 1px solid var(--code-border);
		border-radius: var(--radius-full);
		font-size: var(--text-xs);
		color: var(--code-muted);
		white-space: nowrap;
	}

	.editor {
		min-height: 24.5rem;
		padding-block: var(--space-3);
	}

	.code {
		animation: code-in 0.5s var(--ease-out);
	}

	.code :global(pre.shiki) {
		padding: var(--space-3) var(--space-5);
		font-size: 0.875rem;
		line-height: 1.8;
	}

	.diagnostic-content {
		animation: code-in 0.5s var(--ease-out) 0.15s both;
	}

	.prompt {
		color: var(--brand-pink);
	}

	@keyframes progress {
		from {
			transform: scaleX(0);
		}
		to {
			transform: scaleX(1);
		}
	}

	@keyframes code-in {
		from {
			opacity: 0;
			transform: translateY(6px);
			filter: blur(3px);
		}
	}

	@media (max-width: 720px) {
		.label,
		.lights {
			display: none;
		}

		.editor {
			min-height: 0;
		}

		.code :global(pre.shiki) {
			font-size: 0.75rem;
		}
	}
</style>
