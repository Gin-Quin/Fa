<script lang="ts">
	import GithubIcon from "#lib/components/GithubIcon.svelte";
	import Icon from "#lib/components/Icon.svelte";

	import { reveal } from "./reveal.ts";
</script>

<section class="landing-section">
	<div class="container">
		<div class="cta" {@attach reveal()}>
			<div class="glow" aria-hidden="true"></div>
			<img src="/fa_icon_128x128.webp" alt="" width="72" height="72" />
			<h2>Shape the language <span class="gradient-text">while it's young.</span></h2>
			<p>
				Fa is designed in the open. Read the specification, try the parser, and tell us what you
				would change.
			</p>
			<div class="actions">
				<a class="button button-primary button-large" href="/getting-started/introduction">
					Read the documentation <Icon name="arrowRight" size={16} />
				</a>
				<a
					class="button button-secondary button-large"
					href="https://github.com/Gin-Quin/Fa"
					target="_blank"
					rel="noopener noreferrer"
				>
					<GithubIcon size={16} /> Contribute
				</a>
			</div>
		</div>
	</div>
</section>

<style>
	.cta {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-5);
		padding: var(--space-9) var(--space-6);
		border: 1px solid var(--color-border-strong);
		border-radius: var(--radius-xl);
		background: var(--color-surface);
		text-align: center;
		overflow: hidden;
		isolation: isolate;
	}

	.glow {
		position: absolute;
		inset: auto -10% -60% -10%;
		height: 120%;
		z-index: -1;
		background: radial-gradient(
			ellipse at 50% 100%,
			var(--glow-pink),
			var(--glow-purple) 35%,
			transparent 70%
		);
	}

	.cta::after {
		content: "";
		position: absolute;
		inset: 0;
		z-index: -1;
		background-image:
			linear-gradient(var(--color-grid) 1px, transparent 1px),
			linear-gradient(90deg, var(--color-grid) 1px, transparent 1px);
		background-size: 48px 48px;
		mask-image: radial-gradient(ellipse at 50% 100%, black, transparent 70%);
	}

	img {
		filter: drop-shadow(0 10px 30px var(--glow-pink));
	}

	h2 {
		max-width: 40rem;
		font-family: var(--font-display);
		font-size: var(--text-4xl);
		font-weight: var(--weight-bold);
		letter-spacing: var(--tracking-tight);
		line-height: var(--leading-tight);
		color: var(--color-text);
		text-wrap: balance;
	}

	p {
		max-width: 34rem;
		font-size: var(--text-lg);
		color: var(--color-text-muted);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: var(--space-3);
		margin-top: var(--space-3);
	}
</style>
