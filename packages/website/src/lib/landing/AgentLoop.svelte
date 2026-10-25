<script lang="ts">
	import InlineCode from "./InlineCode.svelte";
	import { reveal } from "./reveal.ts";
	import SectionHeader from "./SectionHeader.svelte";

	let { before, after }: { before: string; after: string } = $props();

	const STEPS = [
		{
			title: "Write",
			text: "The agent writes a call to greet, which requires a session and a logger.",
		},
		{ title: "Compile", text: "The compiler checks every effect of every call, in milliseconds." },
		{ title: "Read", text: "The error says what is wrong, where, and the two ways to fix it." },
		{ title: "Fix", text: "One edit, and it compiles. If it compiles, it works." },
	];

	const DURATION = 3200;

	let step = $state(0);
	let visible = $state(false);
	let manual = $state(false);

	$effect(() => {
		if (!visible || manual) {
			return;
		}
		const timer = setTimeout(() => (step = (step + 1) % STEPS.length), DURATION);
		return () => clearTimeout(timer);
	});

	/** Starts the animation when the section is on screen. */
	function watchVisibility(element: HTMLElement) {
		const observer = new IntersectionObserver(([entry]) => (visible = entry.isIntersecting), {
			threshold: 0.3,
		});
		observer.observe(element);
		return () => observer.disconnect();
	}
</script>

<section class="landing-section" id="agents">
	<div class="container">
		<SectionHeader eyebrow="Built for the agent loop">
			{#snippet title()}
				Write. Compile. Read. <span class="gradient-text">Fix.</span>
			{/snippet}
			An agent codes in a loop. Fa makes every turn of it short: no hidden behavior to guess, and errors
			written to be acted upon.
		</SectionHeader>

		<div class="loop" {@attach watchVisibility} {@attach reveal()}>
			<ol class="steps">
				{#each STEPS as item, index (item.title)}
					<li>
						<button
							type="button"
							class:active={index === step}
							class:done={index < step}
							onclick={() => {
								step = index;
								manual = true;
							}}
						>
							<span class="number">{index + 1}</span>
							<span class="step-text">
								<span class="step-title">{item.title}</span>
								<span class="step-detail"
									><InlineCode text={item.text.replace("greet", "`greet`")} /></span
								>
							</span>
						</button>
					</li>
				{/each}
			</ol>

			<div class="panel terminal with-line-numbers">
				<div class="terminal-header">
					<span class="file">main.fa</span>
					<span class="status" class:ok={step === 3} class:failed={step === 2}>
						{#if step === 1}checking…{:else if step === 2}1 error{:else if step === 3}compiled{:else}editing{/if}
					</span>
				</div>
				{#key step === 3}
					<div class="code">{@html step === 3 ? after : before}</div>
				{/key}
				<div class="diagnostic">
					<p><span class="prompt">$</span> fa check</p>
					{#if step === 1}
						<p class="pending">Checking 1 file<span class="dots"></span></p>
					{:else if step === 2}
						<div class="appear">
							<p>
								<span class="kind-error">error:</span>
								<InlineCode
									text="`greet` requires `Session` and `Logger`, which are not provided"
								/>
							</p>
							<p class="location">&nbsp;&nbsp;--> main.fa:2:2</p>
							<p>
								&nbsp;&nbsp;<span class="help">help:</span>
								<InlineCode
									text="use `provide Session = ...`, or declare `requires session: Session` in `main`"
								/>
							</p>
						</div>
					{:else if step === 3}
						<p class="appear"><span class="kind-success">✓</span> main.fa compiled, 0 errors</p>
					{:else}
						<p class="pending">&nbsp;</p>
					{/if}
				</div>
			</div>
		</div>

		<p class="footnote">
			Diagnostics from the specification. The compiler is in active development.
		</p>
	</div>
</section>

<style>
	.loop {
		display: grid;
		grid-template-columns: minmax(0, 0.8fr) minmax(0, 1.2fr);
		align-items: center;
		gap: var(--space-7);
	}

	.steps {
		display: grid;
		gap: var(--space-2);
		padding: 0;
		list-style: none;
	}

	.steps button {
		display: flex;
		gap: var(--space-4);
		width: 100%;
		padding: var(--space-4);
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		text-align: left;
		transition:
			background-color var(--duration-normal),
			border-color var(--duration-normal);
	}

	.steps button:hover {
		background: var(--color-surface);
	}

	.steps button.active {
		border-color: var(--color-border-strong);
		background: var(--color-surface);
		box-shadow: var(--shadow-md);
	}

	.number {
		display: grid;
		place-items: center;
		flex-shrink: 0;
		width: 2rem;
		height: 2rem;
		border: 1px solid var(--color-border-strong);
		border-radius: 50%;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		transition:
			background var(--duration-normal),
			color var(--duration-normal);
	}

	.active .number,
	.done .number {
		border-color: transparent;
		background: var(--gradient-brand);
		color: #fff;
	}

	.step-text {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.step-title {
		font-weight: var(--weight-semibold);
		color: var(--color-text);
	}

	.step-detail {
		font-size: var(--text-sm);
		color: var(--color-text-muted);
	}

	.step-detail :global(code) {
		font-size: 0.9em;
		color: var(--code-inline-text);
	}

	.terminal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-3) var(--space-5);
		border-bottom: 1px solid var(--code-border);
		background: var(--code-header);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--code-muted);
	}

	.status {
		padding: 0.15rem 0.6rem;
		border-radius: var(--radius-full);
		background: var(--code-border);
	}

	.status.failed {
		color: var(--color-error);
		background: var(--color-error-soft);
	}

	.status.ok {
		color: var(--color-success);
		background: color-mix(in srgb, var(--color-success) 12%, transparent);
	}

	.code {
		min-height: 13rem;
		padding-block: var(--space-2);
		animation: swap 0.5s var(--ease-out);
	}

	.diagnostic {
		min-height: 8.5rem;
	}

	.prompt {
		color: var(--brand-pink);
	}

	.pending {
		color: var(--code-muted);
	}

	.dots::after {
		content: "";
		animation: dots 1.2s steps(4) infinite;
	}

	.appear {
		animation: swap 0.4s var(--ease-out);
	}

	.footnote {
		margin-top: var(--space-5);
		text-align: right;
		font-size: var(--text-xs);
		color: var(--color-text-subtle);
	}

	@keyframes swap {
		from {
			opacity: 0;
			transform: translateY(6px);
		}
	}

	@keyframes dots {
		0% {
			content: "";
		}
		25% {
			content: ".";
		}
		50% {
			content: "..";
		}
		75% {
			content: "...";
		}
	}

	@media (max-width: 900px) {
		.loop {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
