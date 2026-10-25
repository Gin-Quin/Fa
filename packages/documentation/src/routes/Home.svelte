<script lang="ts">
	import FancyButton from "#lib/FancyButton.svelte";
</script>

<svelte:head>
	<title>Fa Documentation</title>
</svelte:head>

<div class="Home">
	<section class="hero" aria-labelledby="home-title">
		<div class="hero-copy">
			<a class="title-button" href="/" aria-label="Fa home">Fa</a>
			<p class="eyebrow">
				&lt;If it <strong>compiles</strong>, it works&gt;
			</p>
			<h1 id="home-title">A programming language for AI agents and humans.</h1>
			<p class="hero-text">
				Agents get strict types, typed effects, clear diagnostics and one way to write each thing.
				Humans get code that reads naturally and runs everywhere: web, server, native, WebAssembly.
			</p>
			<div class="hero-actions">
				<FancyButton href="/getting-started/introduction">Read the docs</FancyButton>
			</div>
		</div>

		<div class="hero-art" aria-hidden="true">
			<img src="/fa_main_icon_512x512.webp" alt="" width="512" height="512" />
		</div>
	</section>

	<section class="feature-cards" aria-label="Fa principles">
		<article class="feature-card feature-card--pink">
			<div class="soft-icon">
				<svg viewBox="0 0 24 24" aria-hidden="true">
					<path d="M12 3l7 3v5c0 4.6-2.9 8.6-7 10-4.1-1.4-7-5.4-7-10V6l7-3z" />
				</svg>
			</div>
			<h2>100% type safety</h2>
			<p>
				No <code>any</code>, no unchecked casts, no <code>null</code> surprises. The compiler is the feedback
				loop.
			</p>
		</article>

		<article class="feature-card feature-card--orange">
			<div class="soft-icon">
				<svg viewBox="0 0 24 24" aria-hidden="true">
					<path d="M13 2L4 14h7l-1 8 10-13h-7l1-7z" />
				</svg>
			</div>
			<h2>Effects in the syntax</h2>
			<p>
				Typed errors, requirements and mutations, à la Effect: every signature tells what a function
				can do.
			</p>
		</article>

		<article class="feature-card feature-card--violet">
			<div class="soft-icon">
				<svg viewBox="0 0 24 24" aria-hidden="true">
					<path d="M8 8l-4 4 4 4M16 8l4 4-4 4M14 4l-4 16" />
				</svg>
			</div>
			<h2>Simple &amp; unambiguous</h2>
			<p>
				A syntax between Rust and TypeScript, one way to write each thing, clear error messages and
				documentation in code.
			</p>
		</article>

		<article class="feature-card feature-card--green">
			<div class="soft-icon">
				<svg viewBox="0 0 24 24" aria-hidden="true">
					<circle cx="12" cy="12" r="9" />
					<path
						d="M3 12h18M12 3c2.6 2.6 3.9 5.6 3.9 9s-1.3 6.4-3.9 9M12 3c-2.6 2.6-3.9 5.6-3.9 9s1.3 6.4 3.9 9"
					/>
				</svg>
			</div>
			<h2>Universal</h2>
			<p>
				Code that reads almost naturally, from the browser to native. One language for your entire
				stack.
			</p>
		</article>
	</section>

	<section class="bento" aria-labelledby="everywhere-title">
		<div class="bento-cell bento-cell--headline">
			<div class="dot-field"></div>
			<h2 id="everywhere-title">
				<span>One language.</span>
				<span>Everywhere.</span>
			</h2>
		</div>

		<article class="bento-cell use-case use-case--web">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<circle cx="12" cy="12" r="9" />
				<path
					d="M3 12h18M12 3c2.6 2.6 3.9 5.6 3.9 9s-1.3 6.4-3.9 9M12 3c-2.6 2.6-3.9 5.6-3.9 9s1.3 6.4 3.9 9"
				/>
			</svg>
			<h3>Web applications</h3>
			<p>Compile to JavaScript and run in the browser.</p>
		</article>

		<article class="bento-cell use-case use-case--tools">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path
					d="M14.7 6.3l3-3 3 3-3 3M16.8 8.4l-8.4 8.4M5 14l5 5-3 3-5-5 3-3zM4 4l5 5M2.7 6.3l3.6-3.6 3 3-3.6 3.6"
				/>
			</svg>
			<h3>Developer tooling &amp; CLIs</h3>
			<p>Write the tools around your code in the same language.</p>
		</article>

		<article class="bento-cell use-case use-case--servers">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path d="M13 2L4 14h7l-1 8 10-13h-7l1-7z" />
			</svg>
			<h3>Servers and backends</h3>
			<p>Run on Bun, Node.js and Deno, or as a native binary.</p>
		</article>

		<article class="bento-cell use-case use-case--embedded">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<rect x="7" y="7" width="10" height="10" rx="1.5" />
				<path d="M9 2v5M15 2v5M9 17v5M15 17v5M2 9h5M2 15h5M17 9h5M17 15h5" />
			</svg>
			<h3>Native &amp; WebAssembly</h3>
			<p>No garbage collector: compile to native code or WebAssembly.</p>
		</article>

		<article class="bento-cell use-case use-case--games">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path
					d="M7 8h10c2.2 0 3.9 1.5 4.3 3.7l.7 4.1c.3 1.9-1.2 3.7-3.1 3.7-1 0-1.9-.5-2.5-1.3l-1.1-1.7H8.7l-1.1 1.7c-.6.8-1.5 1.3-2.5 1.3-1.9 0-3.4-1.8-3.1-3.7l.7-4.1C3.1 9.5 4.8 8 7 8z"
				/>
				<path d="M8 11v4M6 13h4M16.5 12h.1M18.5 15h.1" />
			</svg>
			<h3>Games</h3>
			<p>Share the same code between your game, your tools and your servers.</p>
		</article>

		<article class="bento-cell use-case use-case--code">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path d="M9 7l-5 5 5 5M15 7l5 5-5 5" />
			</svg>
			<p>Fa can also emit <strong>Rust source code</strong></p>
			<small>Pick your targets in the package configuration.</small>
		</article>

		<div class="bento-cell bento-cell--wide">
			<img src="/3d_cube.webp" alt="" width="160" height="160" />
			<div>
				<p>Write once. Target multiple platforms.</p>
				<strong>Tooling, applications and games from one code base.</strong>
			</div>
		</div>
	</section>

	<section class="comparison panel" aria-labelledby="replacement-title">
		<div class="panel-heading">
			<div class="dot-field dot-field--left"></div>
			<h2 id="replacement-title">
				A syntax between <span>TypeScript and Rust</span>
			</h2>
			<div class="dot-field dot-field--right"></div>
		</div>

		<div class="comparison-grid">
			<article>
				<div class="language-heading">
					<span class="language-mark language-mark--ts">TS</span>
					<h3>TypeScript</h3>
				</div>
				<ul>
					<li class="good">Readable and productive</li>
					<li class="bad">Escape hatches: <code>any</code>, unchecked casts</li>
					<li class="bad">Errors are not part of the types</li>
				</ul>
			</article>

			<div class="versus" aria-hidden="true">VS</div>

			<article>
				<div class="language-heading">
					<span class="language-mark language-mark--rust">R</span>
					<h3>Rust</h3>
				</div>
				<ul>
					<li class="good">Fast and memory-safe</li>
					<li class="good">Typed errors</li>
					<li class="bad">Borrow checker and lifetimes to satisfy</li>
				</ul>
			</article>
		</div>
	</section>

	<section class="meet panel" aria-labelledby="meet-title">
		<div class="dot-field dot-field--left"></div>
		<div>
			<h2 id="meet-title">Meet the best of both worlds</h2>
			<p>The <strong>readability</strong> of TypeScript</p>
			<p>The <strong>safety and performance</strong> of Rust</p>
			<p>Plus typed errors, requirements and mutations in every signature</p>
		</div>
		<div class="dot-field dot-field--right"></div>
	</section>

	<section class="performance panel" aria-labelledby="performance-title">
		<div class="performance-title">
			<h2 id="performance-title">Predictable performance</h2>
			<svg viewBox="0 0 120 80" aria-hidden="true">
				<path d="M14 66a46 46 0 1 1 92 0" />
				<path d="M34 66a26 26 0 1 1 52 0" />
				<path d="M60 66l30-44" />
				<circle cx="60" cy="66" r="6" />
			</svg>
		</div>

		<div class="performance-card">
			<div class="dot-field"></div>
			<p>
				Fa compiles to JavaScript, native code, WebAssembly or Rust while remaining simple to reason
				about.
			</p>
			<ul>
				<li>No garbage collector</li>
				<li>No borrow checker</li>
				<li>No unpredictable pauses</li>
			</ul>
		</div>
	</section>

	<section class="memory panel" aria-labelledby="memory-title">
		<div class="panel-heading">
			<h2 id="memory-title">A new take on memory management</h2>
			<p>No garbage collector and no borrow checker, yet 100% memory-safe.</p>
		</div>
		<div class="three-up">
			<article>
				<h3>No garbage collector</h3>
				<p>Values are destroyed at the end of their scope, without pauses.</p>
			</article>
			<article>
				<h3>No borrow checker</h3>
				<p>Safety is part of the type system, without fighting the compiler.</p>
			</article>
			<article>
				<h3>No lifetime annotations</h3>
				<p>
					The compiler tracks which operations invalidate references, and asks you to revalidate
					them.
				</p>
			</article>
		</div>
		<div class="split-bento">
			<div class="split-intro">
				<p>Instead, Fa relies on two core concepts.</p>
			</div>
			<article>
				<h3>Where data lives is part of the type</h3>
				<p>
					A reference to an element of <code>persons</code> has the type
					<code>Person in persons</code>.
				</p>
			</article>
			<article>
				<h3>Relations</h3>
				<p>Safe, explicit relations instead of weak pointers. A relation never dangles.</p>
			</article>
		</div>
	</section>

	<section class="reactivity bento" aria-labelledby="reactivity-title">
		<div class="bento-cell bento-cell--headline">
			<div class="dot-field"></div>
			<h2 id="reactivity-title">
				<span>And more:</span>
				<span>built-in reactivity.</span>
			</h2>
		</div>
		<div class="bento-cell bento-cell--text">
			<p>
				Declare <code>reactive</code> and <code>derived</code> values: the language keeps them in sync.
			</p>
		</div>
		<article class="bento-cell use-case use-case--embedded">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path d="M12 3v3M12 18v3M4.2 7.5l2.6 1.5M17.2 15l2.6 1.5M4.2 16.5l2.6-1.5M17.2 9l2.6-1.5" />
				<circle cx="12" cy="12" r="4" />
			</svg>
			<h3>Compiled</h3>
			<p>Reactive updates are understood by the compiler, not bolted on at runtime.</p>
		</article>
		<article class="bento-cell use-case use-case--web">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path d="M12 5v14M5 12h14" />
				<circle cx="12" cy="12" r="9" />
			</svg>
			<h3>Type-safe</h3>
			<p>Dependencies stay explicit and checked as part of normal language flow.</p>
		</article>
		<article class="bento-cell use-case use-case--tools">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path d="M4 12l5 5L20 6" />
			</svg>
			<h3>Predictable</h3>
			<p>No hidden subscriptions, no framework-specific state model to memorize.</p>
		</article>
	</section>

	<section class="metaprogramming bento" aria-labelledby="metaprogramming-title">
		<div class="bento-cell bento-cell--text">
			<p>If you can write Fa, you can metaprogram in Fa.</p>
		</div>
		<div class="bento-cell bento-cell--headline">
			<div class="dot-field"></div>
			<h2 id="metaprogramming-title">
				<span>Metaprogramming,</span>
				<span>without pain.</span>
			</h2>
		</div>
		<article class="bento-cell use-case use-case--servers">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path d="M13 2L4 14h7l-1 8 10-13h-7l1-7z" />
			</svg>
			<h3>Compile-time execution</h3>
			<p>Run Fa while compiling Fa.</p>
		</article>
		<article class="bento-cell use-case use-case--tools">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path d="M14.7 6.3l3-3 3 3-3 3M16.8 8.4l-8.4 8.4M5 14l5 5-3 3-5-5 3-3z" />
			</svg>
			<h3>Code generation</h3>
			<p>Generate code from the same language and type model you already use.</p>
		</article>
		<article class="bento-cell use-case use-case--code">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path d="M9 7l-5 5 5 5M15 7l5 5-5 5" />
			</svg>
			<h3>No separate DSLs</h3>
			<p>
				Compile-time operations are marked with <code>@</code> and are still plain Fa.
			</p>
		</article>
		<article class="bento-cell use-case use-case--games">
			<svg viewBox="0 0 24 24" aria-hidden="true">
				<path d="M4 4h16v16H4zM8 8h8M8 12h5M8 16h7" />
			</svg>
			<h3>No unreadable syntax trees</h3>
			<p>Work with language concepts instead of manually assembling tree shapes.</p>
		</article>
	</section>

	<section class="final-cta" aria-labelledby="final-title">
		<h2 id="final-title">Designed for AI agents and humans.</h2>
		<p>Fa is in active development. Expect breaking changes.</p>
		<strong>If it compiles, it works.</strong>
		<FancyButton href="/getting-started/introduction">Read the docs</FancyButton>
	</section>
</div>

<style>
	.Home {
		--home-bg: var(--white);
		--surface: var(--home-bg);
		--surface-soft: var(--home-bg);
		--line: #d1d5db;
		--text-soft: color-mix(in srgb, var(--black) 68%, transparent);
		--shadow: color-mix(in srgb, var(--black) 10%, transparent);
		--gradient: linear-gradient(90deg, var(--purpleLight), var(--orangeDark), var(--orangeLight));
		--radius: 0;
		box-sizing: border-box;
		min-height: 100dvh;
		padding: clamp(2rem, 6vw, 5rem) clamp(1rem, 4vw, 2rem) 6rem;
		background: var(--home-bg);
		color: var(--black);
		font-family: "Fira Code", monospace;
		font-size: 16px;
	}

	.Home *,
	.Home *::before,
	.Home *::after {
		box-sizing: border-box;
	}

	:global(html.dark) .Home {
		--home-bg: #000;
		--surface: var(--home-bg);
		--surface-soft: var(--home-bg);
		--line: #3a3a3a;
		--text-soft: rgba(255, 255, 255, 0.72);
		--shadow: rgba(0, 0, 0, 0.48);
	}

	.hero,
	.feature-cards,
	.bento,
	.panel,
	.final-cta {
		width: min(1180px, 100%);
		margin: 0 auto;
	}

	.hero {
		display: grid;
		grid-template-columns: minmax(0, 1.06fr) minmax(280px, 0.94fr);
		align-items: center;
		min-height: min(620px, calc(100dvh - 6rem));
		padding: clamp(1.5rem, 4vw, 3.5rem);
		overflow: hidden;
	}

	.hero-copy {
		position: relative;
		z-index: 2;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1rem;
	}

	.title-button {
		position: relative;
		z-index: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 5.4rem;
		min-height: 3.3rem;
		padding: 0.1rem 0.7rem 0.2rem;
		color: white;
		font-family: "Audiowide", cursive;
		font-size: clamp(2.35rem, 6vw, 4.1rem);
		font-weight: 900;
		line-height: 1;
		text-decoration: none;
	}

	.title-button::before {
		content: "";
		position: absolute;
		inset: 0.35rem -0.55rem 0.2rem;
		z-index: -1;
		transform: skew(-16deg);
		background: var(--gradient);
	}

	.eyebrow,
	.hero-text,
	.meet p,
	.panel-heading p,
	.performance-card p,
	.final-cta p {
		color: var(--text-soft);
	}

	.eyebrow {
		margin: 0;
		font-size: clamp(0.95rem, 1.8vw, 1.35rem);
	}

	.hero h1,
	.bento-cell--headline h2,
	.panel h2,
	.final-cta h2 {
		margin: 0;
		font-weight: 900;
		letter-spacing: 0;
		text-wrap: balance;
	}

	.hero h1 {
		max-width: 760px;
		font-size: clamp(2.25rem, 6vw, 4.6rem);
		line-height: 1.04;
	}

	.hero-text {
		max-width: 650px;
		margin: 0;
		font-size: clamp(1rem, 1.7vw, 1.35rem);
		line-height: 1.75;
	}

	.hero-actions {
		position: relative;
		z-index: 3;
		margin-top: 0.75rem;
	}

	.hero-art {
		position: relative;
		display: grid;
		place-items: center;
		min-height: 430px;
	}

	.hero-art::before {
		content: "";
		position: absolute;
		width: min(36vw, 430px);
		aspect-ratio: 1;
		border-radius: 999px;
		background: radial-gradient(
			circle,
			color-mix(in srgb, var(--orangeLight) 22%, transparent),
			transparent 68%
		);
		filter: blur(28px);
	}

	.hero-art img {
		position: relative;
		z-index: 1;
		width: min(430px, 92%);
		height: auto;
		filter: drop-shadow(0 28px 60px color-mix(in srgb, var(--purpleLight) 28%, transparent));
	}

	.dot-field {
		position: absolute;
		width: 220px;
		height: 170px;
		pointer-events: none;
		opacity: 0.42;
		background-image: radial-gradient(currentColor 1.6px, transparent 1.8px);
		background-size: 22px 22px;
		color: var(--purpleLight);
		mask-image: radial-gradient(circle, black 0 42%, transparent 72%);
	}

	.feature-cards {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 1.1rem;
		margin-top: 1.3rem;
	}

	.feature-card {
		position: relative;
		min-height: 255px;
		padding: 1.55rem;
		border: 1px solid var(--line);
		border-radius: 0;
		background: var(--surface);
		box-shadow: 0 16px 38px color-mix(in srgb, var(--black) 7%, transparent);
		overflow: hidden;
	}

	.feature-card::after {
		content: "";
		position: absolute;
		right: 1.45rem;
		bottom: 1.45rem;
		width: 0.48rem;
		height: 0.48rem;
		background: currentColor;
	}

	.feature-card h2 {
		margin: 1.85rem 0 1.1rem;
		font-size: clamp(1.05rem, 1.8vw, 1.25rem);
		line-height: 1.35;
	}

	.feature-card p {
		margin: 0;
		color: var(--text-soft);
		font-size: 0.98rem;
		line-height: 1.8;
	}

	.feature-card--pink {
		color: var(--redLight);
	}

	.feature-card--orange {
		color: var(--orangeDark);
	}

	.feature-card--violet {
		color: #7c3cff;
	}

	.feature-card--green {
		color: var(--green);
	}

	.soft-icon {
		display: grid;
		place-items: center;
		width: 4.15rem;
		aspect-ratio: 1;
		border-radius: 0.8rem;
		background: color-mix(in srgb, currentColor 13%, var(--surface));
	}

	svg {
		display: block;
		fill: none;
		stroke: currentColor;
		stroke-linecap: round;
		stroke-linejoin: round;
		stroke-width: 1.8;
	}

	.soft-icon svg {
		width: 2.25rem;
	}

	.bento {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		margin-top: 2rem;
		border: 1px solid var(--line);
		background: var(--line);
		gap: 1px;
		overflow: hidden;
	}

	.bento-cell {
		position: relative;
		min-height: 285px;
		padding: clamp(1.35rem, 3vw, 2.4rem);
		background:
			radial-gradient(
				circle at 50% 40%,
				color-mix(in srgb, var(--black) 3%, transparent),
				transparent 13rem
			),
			var(--surface);
		overflow: hidden;
	}

	.bento-cell--headline {
		grid-column: span 2;
		display: flex;
		align-items: center;
	}

	.bento-cell--headline .dot-field {
		left: 1rem;
		top: 1rem;
	}

	.bento-cell--headline h2 {
		position: relative;
		z-index: 1;
		display: grid;
		gap: 0.35rem;
		background: var(--gradient);
		-webkit-background-clip: text;
		background-clip: text;
		color: transparent;
		font-size: clamp(2rem, 4vw, 3.15rem);
		line-height: 1.13;
	}

	.use-case {
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 1.15rem;
	}

	.use-case svg {
		width: 4.4rem;
		color: var(--orangeDark);
	}

	.use-case--web svg,
	.use-case--tools svg,
	.use-case--games svg,
	.use-case--code svg {
		color: var(--purpleLight);
	}

	.use-case h3 {
		max-width: 17rem;
		margin: 0;
		font-size: clamp(1.3rem, 2vw, 1.65rem);
		line-height: 1.35;
	}

	.use-case p,
	.use-case small {
		max-width: 19rem;
		margin: 0;
		color: var(--text-soft);
		font-size: 1rem;
		line-height: 1.65;
	}

	.bento-cell--text {
		display: grid;
		place-items: center;
		text-align: center;
	}

	.bento-cell--text p {
		max-width: 28rem;
		margin: 0;
		color: var(--text-soft);
		font-size: clamp(1.2rem, 2.3vw, 1.7rem);
		font-weight: 700;
		line-height: 1.45;
		text-wrap: balance;
	}

	.use-case--code {
		text-align: center;
		align-items: center;
	}

	.bento-cell--wide {
		grid-column: span 4;
		display: grid;
		grid-template-columns: 230px minmax(0, 1fr);
		align-items: center;
		min-height: 200px;
	}

	.bento-cell--wide img {
		width: min(9rem, 70%);
		height: auto;
		justify-self: center;
		filter: drop-shadow(0 18px 36px color-mix(in srgb, var(--purpleLight) 18%, transparent));
	}

	.bento-cell--wide p {
		margin: 0 0 1.1rem;
		font-size: clamp(1.35rem, 2.6vw, 2rem);
	}

	.bento-cell--wide strong,
	.panel-heading span,
	.meet h2,
	.performance-title h2,
	.final-cta h2 {
		background: var(--gradient);
		-webkit-background-clip: text;
		background-clip: text;
		color: transparent;
	}

	.bento-cell--wide strong {
		font-size: clamp(1.08rem, 2.1vw, 1.55rem);
	}

	.panel {
		margin-top: 2rem;
		border: 1px solid var(--line);
		background: var(--surface);
		box-shadow: 0 16px 44px color-mix(in srgb, var(--black) 6%, transparent);
		overflow: hidden;
	}

	.panel-heading {
		position: relative;
		display: grid;
		place-items: center;
		min-height: 155px;
		padding: 2rem;
		border-bottom: 1px solid var(--line);
		text-align: center;
		overflow: hidden;
	}

	.panel-heading h2,
	.meet h2 {
		font-size: clamp(1.75rem, 3.5vw, 2.7rem);
		line-height: 1.25;
	}

	.panel-heading p {
		max-width: 720px;
		margin: 1rem 0 0;
		font-size: clamp(1rem, 1.7vw, 1.25rem);
		line-height: 1.65;
	}

	.three-up,
	.split-bento {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 1px;
		background: var(--line);
	}

	.three-up article,
	.split-bento > * {
		min-height: 210px;
		padding: clamp(1.5rem, 3vw, 2.6rem);
		background: var(--surface);
	}

	.three-up h3,
	.split-bento h3 {
		margin: 0 0 1rem;
		font-size: clamp(1.25rem, 2vw, 1.55rem);
		line-height: 1.35;
	}

	.three-up p,
	.split-bento p {
		margin: 0;
		color: var(--text-soft);
		font-size: 1rem;
		line-height: 1.7;
	}

	.split-intro {
		display: grid;
		place-items: center;
		text-align: center;
	}

	.split-intro p {
		max-width: 16rem;
		font-size: clamp(1.15rem, 2vw, 1.45rem);
		font-weight: 900;
		color: var(--black);
	}

	.split-bento h3 {
		background: var(--gradient);
		-webkit-background-clip: text;
		background-clip: text;
		color: transparent;
	}

	.panel-heading .dot-field--left,
	.meet .dot-field--left {
		left: 2rem;
		top: 1.4rem;
	}

	.panel-heading .dot-field--right,
	.meet .dot-field--right {
		right: 2rem;
		top: 1.4rem;
		color: var(--orangeLight);
	}

	.comparison-grid {
		position: relative;
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
	}

	.comparison-grid article {
		padding: clamp(2rem, 5vw, 4rem) clamp(1.5rem, 6vw, 8rem);
	}

	.comparison-grid article:first-child {
		border-right: 1px solid var(--line);
	}

	.language-heading {
		display: flex;
		align-items: center;
		gap: 1.2rem;
		margin-bottom: 2rem;
	}

	.language-heading h3 {
		margin: 0;
		font-size: clamp(1.5rem, 3vw, 2rem);
	}

	.language-mark {
		display: inline-grid;
		place-items: center;
		width: 3.4rem;
		aspect-ratio: 1;
		font-weight: 900;
		font-size: 1.55rem;
	}

	.language-mark--ts {
		border: 2px solid var(--purpleLight);
		border-radius: 0.45rem;
		color: var(--purpleLight);
	}

	.language-mark--rust {
		border: 4px dotted var(--orangeDark);
		border-radius: 999px;
		color: var(--orangeDark);
	}

	.comparison ul,
	.performance-card ul {
		display: grid;
		gap: 1.2rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.comparison li,
	.performance-card li {
		position: relative;
		padding-left: 2.2rem;
		color: var(--text-soft);
		font-size: clamp(1rem, 2vw, 1.28rem);
		line-height: 1.45;
	}

	.comparison li::before,
	.performance-card li::before {
		position: absolute;
		left: 0;
		top: 0;
		font-weight: 900;
	}

	.comparison li.good::before {
		content: "✓";
		color: var(--green);
	}

	.comparison li.bad::before {
		content: "×";
		color: var(--redLight);
	}

	.versus {
		position: relative;
		z-index: 1;
		align-self: center;
		width: 3.8rem;
		margin: 0 -1.9rem;
		aspect-ratio: 1;
		display: grid;
		place-items: center;
		border: 1px solid var(--line);
		border-radius: 999px;
		background: var(--surface);
		font-weight: 900;
	}

	.meet {
		position: relative;
		display: grid;
		place-items: center;
		min-height: 315px;
		padding: clamp(2rem, 5vw, 4rem);
		text-align: center;
	}

	.meet h2 {
		margin-bottom: 1.8rem;
	}

	.meet p {
		margin: 0.8rem 0 0;
		font-size: clamp(1.1rem, 2.2vw, 1.6rem);
		line-height: 1.45;
	}

	.meet p:last-child {
		color: color-mix(in srgb, var(--text-soft) 70%, transparent);
	}

	.performance {
		display: grid;
		grid-template-columns: 0.95fr 1.65fr;
		gap: 1px;
		background: var(--line);
	}

	.performance-title,
	.performance-card {
		position: relative;
		background: var(--surface);
		padding: clamp(2rem, 5vw, 4rem);
	}

	.performance-title {
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 3rem;
	}

	.performance-title h2 {
		font-size: clamp(2rem, 4vw, 3.25rem);
		line-height: 1.28;
	}

	.performance-title svg {
		width: min(260px, 88%);
		color: var(--orangeLight);
		stroke-width: 4;
	}

	.performance-card {
		margin: clamp(1rem, 3vw, 2.5rem);
		border: 1px solid var(--line);
		border-radius: 14px;
		overflow: hidden;
	}

	.performance-card .dot-field {
		right: 0;
		top: 0;
		color: var(--gray);
		opacity: 0.22;
	}

	.performance-card p {
		position: relative;
		margin: 0 0 2rem;
		padding: 0 0 1.7rem 2rem;
		border-bottom: 1px solid var(--line);
		font-size: clamp(1.1rem, 2vw, 1.45rem);
		line-height: 1.55;
	}

	.performance-card p::before {
		content: "";
		position: absolute;
		left: 0;
		top: 0.2rem;
		width: 2px;
		height: 5.3rem;
		background: var(--gradient);
	}

	.performance-card li::before {
		content: "⊘";
		color: var(--orangeDark);
		text-shadow: 0 0 10px color-mix(in srgb, var(--redLight) 55%, transparent);
	}

	.final-cta {
		display: grid;
		place-items: center;
		gap: 1.2rem;
		margin-top: 2rem;
		padding: clamp(3rem, 7vw, 5rem) 1.5rem;
		text-align: center;
	}

	.final-cta h2 {
		font-size: clamp(2rem, 4vw, 3.25rem);
	}

	.final-cta p {
		max-width: 680px;
		margin: 0 0 0.75rem;
		font-size: clamp(1rem, 1.7vw, 1.2rem);
		line-height: 1.7;
	}

	.final-cta strong {
		display: block;
		margin-bottom: 1rem;
		background: var(--gradient);
		-webkit-background-clip: text;
		background-clip: text;
		color: transparent;
		font-size: clamp(1.8rem, 3.4vw, 2.65rem);
	}

	@media (max-width: 920px) {
		.hero,
		.performance {
			grid-template-columns: 1fr;
		}

		.hero {
			min-height: auto;
		}

		.hero-copy {
			align-items: center;
			text-align: center;
		}

		.hero-art {
			order: -1;
			min-height: 300px;
		}

		.hero-art img {
			width: min(310px, 76vw);
		}

		.feature-cards,
		.bento,
		.three-up,
		.split-bento {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}

		.bento-cell--headline,
		.bento-cell--wide {
			grid-column: span 2;
		}

		.bento-cell--wide {
			grid-template-columns: 1fr;
			gap: 1.5rem;
			text-align: center;
		}

		.comparison-grid {
			grid-template-columns: 1fr;
		}

		.comparison-grid article:first-child {
			border-right: none;
			border-bottom: 1px solid var(--line);
		}

		.versus {
			position: absolute;
			left: 50%;
			top: 50%;
			transform: translate(-50%, -50%);
			margin: 0;
		}
	}

	@media (max-width: 640px) {
		.Home {
			padding-inline: 0.75rem;
		}

		.hero {
			padding-inline: 0.25rem;
		}

		.feature-cards,
		.bento,
		.three-up,
		.split-bento {
			grid-template-columns: 1fr;
		}

		.bento-cell,
		.bento-cell--headline,
		.bento-cell--wide {
			grid-column: span 1;
		}

		.comparison-grid article {
			padding: 2rem 1.3rem;
		}

		.panel-heading .dot-field,
		.meet .dot-field {
			display: none;
		}
	}
</style>
