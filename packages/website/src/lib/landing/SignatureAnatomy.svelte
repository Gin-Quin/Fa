<script lang="ts">
	import { reveal } from "./reveal.ts";
	import SectionHeader from "./SectionHeader.svelte";
	import type { SignatureSegment } from "./types.ts";

	type Part = { kind: string; label: string; title: string; text: string; call: string };

	let { segments, body, parts }: { segments: SignatureSegment[]; body: string; parts: Part[] } =
		$props();

	let active = $state("throws");

	const part = $derived(parts.find((candidate) => candidate.kind === active) ?? parts[0]);
</script>

<section class="landing-section" id="signatures">
	<div class="container">
		<SectionHeader eyebrow="Effects in the syntax">
			{#snippet title()}
				The signature tells <span class="gradient-text">everything</span>
			{/snippet}
			What a function returns, how it fails, what it needs and what it changes: all of it is in its signature,
			checked by the compiler. An agent reads one line and knows what a call can do.
		</SectionHeader>

		<div class="anatomy" {@attach reveal()}>
			<div class="panel code">
				<pre class="signature" data-active={active}><code
						>{#each segments as segment, index (index)}{#if segment.kind}<button
							type="button"
							class="segment segment-{segment.kind}"
							class:active={segment.kind === active}
							onmouseenter={() => (active = segment.kind ?? active)}
							onfocus={() => (active = segment.kind ?? active)}
							onclick={() => (active = segment.kind ?? active)}
							>{#each segment.tokens as token, tokenIndex (tokenIndex)}<span
								style:color={token.color}
								style:font-weight={token.bold ? 600 : undefined}>{token.content}</span
							>{/each}</button
						>{:else}{#each segment.tokens as token, tokenIndex (tokenIndex)}<span
							style:color={token.color}>{token.content}</span
						>{/each}{/if}{/each}</code
					></pre>
				<div class="body">{@html body}</div>
			</div>

			<div class="tabs" role="tablist" aria-label="Parts of the signature">
				{#each parts as item (item.kind)}
					<button
						type="button"
						role="tab"
						class="tab tab-{item.kind}"
						aria-selected={item.kind === active}
						onclick={() => (active = item.kind)}
						onmouseenter={() => (active = item.kind)}
					>
						{item.label}
					</button>
				{/each}
			</div>

			<div class="explanation">
				{#key active}
					<div class="text">
						<h3>{part.title}</h3>
						<p>{part.text}</p>
					</div>
					<div class="panel call">
						<p class="call-label">At the call site</p>
						{@html part.call}
					</div>
				{/key}
			</div>
		</div>
	</div>
</section>

<style>
	.anatomy {
		display: grid;
		gap: var(--space-5);
	}

	.code {
		padding-block: var(--space-4);
	}

	.signature {
		overflow-x: auto;
		padding: var(--space-3) var(--space-6);
		font-size: clamp(0.8rem, 0.6rem + 0.45vw, 1rem);
		line-height: 2.2;
		color: var(--code-text);
		white-space: pre;
	}

	.segment {
		position: relative;
		padding: 0.25em 0.15em;
		border-radius: var(--radius-sm);
		font: inherit;
		white-space: pre;
		cursor: pointer;
		transition:
			background-color var(--duration-normal),
			box-shadow var(--duration-normal),
			opacity var(--duration-normal);
	}

	.segment-returns {
		--part: var(--effect-returns);
	}
	.segment-throws {
		--part: var(--effect-throws);
	}
	.segment-requires {
		--part: var(--effect-requires);
	}
	.segment-mutates {
		--part: var(--effect-mutates);
	}

	.segment.active {
		background: color-mix(in srgb, var(--part) 16%, transparent);
		box-shadow:
			inset 0 0 0 1px color-mix(in srgb, var(--part) 55%, transparent),
			0 0 28px -4px color-mix(in srgb, var(--part) 55%, transparent);
	}

	.segment:not(.active):hover {
		background: rgb(255 255 255 / 0.05);
	}

	.body :global(pre.shiki) {
		padding: 0 var(--space-6) var(--space-3);
		font-size: clamp(0.8rem, 0.6rem + 0.45vw, 1rem);
		line-height: 1.9;
		opacity: 0.55;
	}

	/* ─── Tabs ─────────────────────────────────────────── */

	.tabs {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: var(--space-2);
	}

	.tab {
		--part: var(--color-text-muted);
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-4);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-full);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		background: var(--color-surface);
		transition:
			border-color var(--duration-normal),
			color var(--duration-normal),
			background-color var(--duration-normal);
	}

	.tab::before {
		content: "";
		width: 8px;
		height: 8px;
		border-radius: 2px;
		background: var(--part);
	}

	.tab-returns {
		--part: var(--effect-returns);
	}
	.tab-throws {
		--part: var(--effect-throws);
	}
	.tab-requires {
		--part: var(--effect-requires);
	}
	.tab-mutates {
		--part: var(--effect-mutates);
	}

	.tab[aria-selected="true"] {
		border-color: color-mix(in srgb, var(--part) 60%, transparent);
		color: var(--color-text);
		background: color-mix(in srgb, var(--part) 10%, var(--color-surface));
	}

	/* ─── Explanation ──────────────────────────────────── */

	.explanation {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1.15fr);
		align-items: center;
		gap: var(--space-6);
		margin-top: var(--space-3);
	}

	.text {
		animation: appear 0.45s var(--ease-out);
	}

	.text h3 {
		font-size: var(--text-xl);
		font-weight: var(--weight-semibold);
		letter-spacing: var(--tracking-snug);
		color: var(--color-text);
	}

	.text p {
		margin-top: var(--space-3);
		font-size: var(--text-md);
		color: var(--color-text-muted);
	}

	.call {
		box-shadow: var(--shadow-md);
		animation: appear 0.45s var(--ease-out) 0.05s both;
	}

	.call-label {
		padding: var(--space-3) var(--space-5) 0;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--code-muted);
	}

	@keyframes appear {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}

	@media (max-width: 820px) {
		.explanation {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
