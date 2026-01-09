<script lang="ts">
	import { colors } from "../colors";
	import EnterTheMatrix from "./EnterTheMatrix.svelte";
	import FancyButton from "./FancyButton.svelte";
</script>

<column class="Home" style:gap="3rem">
	<!-- Hero Section -->
	<row class="intro" style:justify-content="space-between" style:gap="5rem">
		<row class="start" style:gap="1rem">
			<img src="/fa_main_icon_500x500.webp" alt="FA Logo" width="220" />
			<div class="description">
				<div class="title">Fa</div>
				<div class="subtitle">
					The <b>universal</b><br />programming language
				</div>
			</div>
		</row>

		<div class="end">
			<FancyButton href="/getting-started/introduction">Get Started</FancyButton
			>
		</div>
	</row>

	<!-- Main Tagline -->
	<div class="hero-tagline">
		<div class="tagline-main">TypeScript at the speed of Rust.</div>
		<div class="tagline-sub">Rust with TypeScript's simplicity.</div>
	</div>

	<div class="hero-description">
		One coherent, safe, high-performance language — from web applications to
		tooling, systems, and beyond.
	</div>

	<div style:padding="2rem 0">
		<EnterTheMatrix />
	</div>

	<!-- One Language Everywhere -->
	<section class="section">
		<h2 class="section-title">One language. Everywhere.</h2>
		<p class="section-intro">
			Fa is built to be <strong>universal by design</strong>.
		</p>
		<div class="features-grid">
			<div class="feature-card">
				<div class="feature-icon">🌐</div>
				<div class="feature-title">Web applications</div>
			</div>
			<div class="feature-card">
				<div class="feature-icon">🛠️</div>
				<div class="feature-title">Developer tooling & CLIs</div>
			</div>
			<div class="feature-card">
				<div class="feature-icon">⚡</div>
				<div class="feature-title">Servers and backends</div>
			</div>
			<div class="feature-card">
				<div class="feature-icon">🔧</div>
				<div class="feature-title">Embedded & systems software</div>
			</div>
			<div class="feature-card">
				<div class="feature-icon">🎮</div>
				<div class="feature-title">Game engines and simulations</div>
			</div>
		</div>
		<div class="highlight-box">
			Write once. Target multiple runtimes.<br />
			No rewrites. No paradigm switches.
		</div>
	</section>

	<!-- Replacement for TypeScript and Rust -->
	<section class="section comparison-section">
		<h2 class="section-title">
			A replacement for TypeScript <em>and</em> Rust
		</h2>
		<p class="section-intro">Today, developers are forced to choose:</p>

		<div class="comparison-grid">
			<div class="comparison-card ts-card">
				<h3>TypeScript</h3>
				<div class="comparison-pros">✓ Productive</div>
				<div class="comparison-cons">✗ Slow and unsafe at runtime</div>
			</div>
			<div class="comparison-card rust-card">
				<h3>Rust</h3>
				<div class="comparison-pros">✓ Fast and safe</div>
				<div class="comparison-cons">✗ Complex and hard to iterate</div>
			</div>
		</div>

		<div class="fa-solution">
			<h3>Fa removes that tradeoff.</h3>
			<ul>
				<li>The <strong>ergonomics of TypeScript</strong></li>
				<li>The <strong>performance and safety of Rust</strong></li>
				<li>Without the downsides of either</li>
			</ul>
		</div>
	</section>

	<!-- Performance Section -->
	<section class="section">
		<h2 class="section-title">Performance without compromise</h2>
		<p class="section-intro">
			Fa compiles to efficient low-level code while remaining simple to reason
			about.
		</p>

		<div class="benefits-list">
			<div class="benefit-item">
				<span class="benefit-icon">🚫</span>
				<span>No runtime garbage collector</span>
			</div>
			<div class="benefit-item">
				<span class="benefit-icon">🚫</span>
				<span>No hidden costs</span>
			</div>
			<div class="benefit-item">
				<span class="benefit-icon">🚫</span>
				<span>No unpredictable pauses</span>
			</div>
		</div>

		<div class="highlight-box performance-box">
			<strong>Predictable performance, by default.</strong>
		</div>
	</section>

	<!-- Memory Model Section -->
	<section class="section memory-section">
		<h2 class="section-title">A new memory model — simple, safe, powerful</h2>
		<p class="section-intro">
			Fa introduces a <strong>unique memory management system</strong>.
		</p>

		<div class="no-list">
			<div class="no-item">❌ No garbage collector</div>
			<div class="no-item">❌ No complex borrow-checker rules</div>
			<div class="no-item">❌ No lifetime annotations everywhere</div>
		</div>

		<div class="memory-concepts">
			<p class="concepts-intro">Instead, Fa relies on two core concepts:</p>

			<div class="concept-card">
				<h3>Owned References</h3>
				<p>Ownership is explicit, local, and intuitive.</p>
			</div>

			<div class="concept-card">
				<h3>Relations</h3>
				<p>
					Lifetimes are expressed declaratively through relationships — not
					inferred through complex compiler logic.
				</p>
			</div>
		</div>

		<div class="result-box">
			<h4>The result:</h4>
			<ul>
				<li>Memory safety by construction</li>
				<li>A clear mental model</li>
				<li>Compiler rules you can actually understand</li>
			</ul>
		</div>
	</section>

	<!-- Reactivity Section -->
	<section class="section reactivity-section">
		<h2 class="section-title">Built-in reactivity — at the language level</h2>
		<p class="section-intro">
			Fa is the <strong
				>first programming language with reactivity built into its core
				semantics</strong
			>.
		</p>

		<div class="no-list">
			<div class="no-item">❌ No frameworks</div>
			<div class="no-item">❌ No hooks</div>
			<div class="no-item">❌ No magic conventions</div>
		</div>

		<div class="reactivity-benefits">
			<p>Reactivity in Fa is:</p>
			<div class="benefits-grid">
				<div class="benefit-badge">statically analyzable</div>
				<div class="benefit-badge">type-safe</div>
				<div class="benefit-badge">predictable</div>
				<div class="benefit-badge">composable</div>
			</div>
		</div>

		<div class="highlight-box reactivity-box">
			State changes propagate <strong
				>because the language understands them</strong
			>,<br />
			not because a library tries to guess.
		</div>
	</section>

	<!-- Metaprogramming Section -->
	<section class="section">
		<h2 class="section-title">Metaprogramming, without pain</h2>
		<p class="section-intro">
			Fa makes metaprogramming <strong>boringly simple</strong>.
		</p>

		<div class="benefits-list">
			<div class="benefit-item">
				<span class="benefit-icon">⚡</span>
				<span>Compile-time execution</span>
			</div>
			<div class="benefit-item">
				<span class="benefit-icon">🔨</span>
				<span>Code generation without macros-from-hell</span>
			</div>
			<div class="benefit-item">
				<span class="benefit-icon">🎯</span>
				<span>No separate DSLs</span>
			</div>
			<div class="benefit-item">
				<span class="benefit-icon">📖</span>
				<span>No unreadable syntax trees</span>
			</div>
		</div>

		<div class="highlight-box">
			If you can write Fa, you can metaprogram in Fa.
		</div>
	</section>

	<!-- One Mental Model -->
	<section class="section unified-section">
		<h2 class="section-title">One mental model, end to end</h2>
		<p class="section-intro">Fa eliminates accidental complexity.</p>

		<div class="unified-grid">
			<div class="unified-item">One type system</div>
			<div class="unified-item">One language</div>
			<div class="unified-item">One toolchain</div>
			<div class="unified-item">One way to reason about correctness</div>
		</div>

		<div class="no-context-switch">
			<p>No context switching between:</p>
			<ul>
				<li>"safe vs unsafe"</li>
				<li>"runtime vs compile-time"</li>
				<li>"frontend vs backend"</li>
			</ul>
		</div>
	</section>

	<!-- Designed for the Future -->
	<section class="section future-section">
		<h2 class="section-title">
			Designed for humans and AIs. Built for the future.
		</h2>

		<div class="future-grid">
			<div class="future-item">Strongly typed</div>
			<div class="future-item">Statically compiled</div>
			<div class="future-item">Fully open source</div>
			<div class="future-item">Vendor-neutral</div>
			<div class="future-item">Designed for long-term maintainability</div>
		</div>

		<div class="foundation-box">
			<p class="foundation-text">This is not an experiment.</p>
			<p class="foundation-emphasis">It's a foundation.</p>
		</div>
	</section>

	<!-- Status Section -->
	<section class="section status-section">
		<h2 class="section-title">Status</h2>
		<p class="section-intro">Fa is under active development.</p>

		<div class="status-box">
			The goal is a <strong>stable v1 focused on web and tooling</strong>, with
			additional backends following.
		</div>

		<div class="belief-box">
			<p>If you believe programming languages should be:</p>
			<ul>
				<li>simpler</li>
				<li>safer</li>
				<li>faster</li>
				<li>more unified</li>
			</ul>
			<p class="belief-conclusion">Then Fa is for you.</p>
		</div>
	</section>

	<!-- CTA Section -->
	<section class="cta-section">
		<FancyButton href="/getting-started/introduction"
			>Get Started with Fa</FancyButton
		>
	</section>
</column>

<style>
	.Home {
		padding-bottom: 4rem;
		position: relative;
		/* Grid background */
		background-image:
			linear-gradient(to right, rgba(200, 200, 200, 0.15) 1px, transparent 1px),
			linear-gradient(to bottom, rgba(200, 200, 200, 0.15) 1px, transparent 1px);
		background-size: 120px 120px;
	}

	.intro {
		width: 800px;
		max-width: 100%;
		padding-top: 4rem;
		margin: auto;
	}

	.title {
		font-size: 3.75rem;
		font-family: "Playwrite IT Moderna", cursive;
		font-optical-sizing: auto;
		letter-spacing: 0.02em;
		font-style: normal;
		background: linear-gradient(45deg, #d900d5, #ff5943, #ed8600);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.subtitle {
		font-size: 1.5rem;
	}

	/* Hero Tagline */
	.hero-tagline {
		text-align: center;
		margin-top: 2rem;
		font-size: 2rem;
		line-height: 1.4;
	}

	.tagline-main {
		font-weight: 700;
		background: linear-gradient(135deg, #ff5943ff, #ed8600ff);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.tagline-sub {
		font-weight: 700;
		background: linear-gradient(135deg, #d900d5ff, #e80059);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.hero-description {
		text-align: center;
		font-size: 1.2rem;
		max-width: 800px;
		margin: 1rem auto;
		opacity: 0.9;
	}

	/* Sections - BRUTALIST: transparent backgrounds, colored borders */
	.section {
		max-width: 900px;
		width: 100%;
		margin: 4rem auto;
		padding: 2rem;
	}

	.section-title {
		font-size: 2.5rem;
		font-weight: 700;
		text-align: center;
		margin-bottom: 1.5rem;
		background: linear-gradient(90deg, #d900d5ff, #ff5943ff, #ed8600ff);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.section-intro {
		text-align: center;
		font-size: 1.2rem;
		margin-bottom: 2rem;
		opacity: 0.9;
	}

	/* Features Grid - BRUTALIST: transparent backgrounds, colored borders */
	.features-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1.5rem;
		margin: 2rem 0;
	}

	.feature-card {
		padding: 1.5rem;
		border-radius: 0;
		background: transparent;
		border: 3px solid #d900d5ff;
		text-align: center;
		transition: all 0.3s ease;
	}

	.feature-card:hover {
		border-color: #ff5943ff;
		transform: translateY(-4px);
	}

	.feature-icon {
		font-size: 2.5rem;
		margin-bottom: 0.5rem;
	}

	.feature-title {
		font-size: 1.1rem;
		font-weight: 600;
	}

	/* Highlight Boxes - BRUTALIST: transparent backgrounds, colored borders */
	.highlight-box {
		margin: 2rem auto;
		padding: 1.5rem 2rem;
		background: transparent;
		border: 3px solid #d900d5ff;
		border-radius: 0;
		font-size: 1.2rem;
		text-align: center;
		line-height: 1.6;
	}

	/* Comparison Section - BRUTALIST: transparent backgrounds, colored borders */
	.comparison-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 2rem;
		margin: 2rem 0;
	}

	.comparison-card {
		padding: 2rem;
		border-radius: 0;
		text-align: center;
		background: transparent;
	}

	.ts-card {
		border: 3px solid #ff5943ff;
	}

	.rust-card {
		border: 3px solid #ed8600ff;
	}

	.comparison-card h3 {
		font-size: 1.8rem;
		margin-bottom: 1rem;
	}

	.comparison-pros {
		color: #4ade80;
		margin: 0.5rem 0;
		font-size: 1.1rem;
	}

	.comparison-cons {
		color: #f87171;
		margin: 0.5rem 0;
		font-size: 1.1rem;
	}

	.fa-solution {
		margin-top: 3rem;
		padding: 2rem;
		background: transparent;
		border: 3px solid;
		border-image: linear-gradient(135deg, #d900d5ff, #e80059) 1;
		border-radius: 0;
		text-align: center;
	}

	.fa-solution h3 {
		font-size: 1.8rem;
		margin-bottom: 1rem;
		color: #d900d5ff;
	}

	.fa-solution ul {
		list-style: none;
		padding: 0;
		font-size: 1.2rem;
		line-height: 2;
	}

	/* Benefits List - BRUTALIST: transparent backgrounds, colored borders, no gaps */
	.benefits-list {
		display: flex;
		flex-direction: column;
		gap: 0;
		margin: 2rem 0;
	}

	.benefit-item {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1rem 1.5rem;
		background: transparent;
		border: 2px solid #d900d5ff;
		border-bottom: none;
		border-radius: 0;
		font-size: 1.1rem;
		transition: all 0.2s ease;
	}

	.benefit-item:last-child {
		border-bottom: 2px solid #d900d5ff;
	}

	.benefit-item:hover {
		background: rgba(217, 0, 213, 0.1);
		transform: translateX(8px);
	}

	.benefit-icon {
		font-size: 1.5rem;
	}

	/* No List - BRUTALIST: transparent backgrounds, colored borders, no gaps */
	.no-list {
		display: flex;
		flex-direction: column;
		gap: 0;
		margin: 2rem 0;
	}

	.no-item {
		padding: 1rem;
		font-size: 1.2rem;
		text-align: center;
		background: transparent;
		border: 2px solid #e80059;
		border-bottom: none;
		border-radius: 0;
	}

	.no-item:last-child {
		border-bottom: 2px solid #e80059;
	}

	/* Memory Concepts - BRUTALIST: transparent backgrounds, colored borders */
	.memory-concepts {
		margin: 2rem 0;
	}

	.concepts-intro {
		text-align: center;
		font-size: 1.2rem;
		margin-bottom: 1.5rem;
		font-weight: 600;
	}

	.concept-card {
		margin: 1.5rem 0;
		padding: 1.5rem;
		background: transparent;
		border: 3px solid #ed8600ff;
		border-radius: 0;
	}

	.concept-card h3 {
		font-size: 1.5rem;
		color: #ed8600ff;
		margin-bottom: 0.5rem;
	}

	.concept-card p {
		font-size: 1.1rem;
		opacity: 0.9;
	}

	.result-box {
		margin: 2rem 0;
		padding: 2rem;
		background: transparent;
		border: 3px solid #d900d5ff;
		border-radius: 0;
	}

	.result-box h4 {
		font-size: 1.4rem;
		margin-bottom: 1rem;
		text-align: center;
		color: #d900d5ff;
	}

	.result-box ul {
		list-style: none;
		padding: 0;
		font-size: 1.1rem;
		line-height: 2;
	}

	.result-box li::before {
		content: "✓ ";
		color: #4ade80;
		font-weight: bold;
		margin-right: 0.5rem;
	}

	/* Reactivity Section - BRUTALIST: transparent backgrounds, colored borders, no gaps */
	.reactivity-benefits {
		margin: 2rem 0;
		text-align: center;
	}

	.reactivity-benefits > p {
		font-size: 1.2rem;
		margin-bottom: 1.5rem;
		font-weight: 600;
	}

	.benefits-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: 0;
		margin: 1rem 0;
	}

	.benefit-badge {
		padding: 1rem;
		background: transparent;
		border: 2px solid;
		border-image: linear-gradient(135deg, #d900d5ff, #e80059) 1;
		border-radius: 0;
		font-weight: 600;
		font-size: 1.1rem;
		text-align: center;
	}

	/* Unified Section - BRUTALIST: transparent backgrounds, colored borders */
	.unified-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1.5rem;
		margin: 2rem 0;
	}

	.unified-item {
		padding: 1.5rem;
		background: transparent;
		border: 3px solid;
		border-image: linear-gradient(135deg, #ff5943ff, #ed8600ff) 1;
		border-radius: 0;
		text-align: center;
		font-size: 1.2rem;
		font-weight: 600;
	}

	.no-context-switch {
		margin: 2rem 0;
		padding: 1.5rem;
		background: transparent;
		border: 3px solid #d900d5ff;
		border-radius: 0;
		text-align: center;
	}

	.no-context-switch p {
		font-size: 1.2rem;
		font-weight: 600;
		margin-bottom: 1rem;
	}

	.no-context-switch ul {
		list-style: none;
		padding: 0;
		font-size: 1.1rem;
		line-height: 1.8;
	}

	/* Future Section - BRUTALIST: transparent backgrounds, colored borders */
	.future-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1.5rem;
		margin: 2rem 0;
	}

	.future-item {
		padding: 1.5rem;
		background: transparent;
		border: 3px solid #d900d5ff;
		border-radius: 0;
		text-align: center;
		font-size: 1.1rem;
		font-weight: 600;
	}

	.foundation-box {
		margin: 3rem 0;
		padding: 2rem;
		text-align: center;
		background: transparent;
		border: 3px solid;
		border-image: linear-gradient(135deg, #d900d5ff, #e80059) 1;
		border-radius: 0;
	}

	.foundation-text {
		font-size: 1.3rem;
		margin-bottom: 0.5rem;
	}

	.foundation-emphasis {
		font-size: 2rem;
		font-weight: 700;
		background: linear-gradient(90deg, #d900d5ff, #e80059);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	/* Status Section - BRUTALIST: transparent backgrounds, colored borders */
	.status-box {
		padding: 1.5rem 2rem;
		background: transparent;
		border: 3px solid #ed8600ff;
		border-radius: 0;
		font-size: 1.2rem;
		text-align: center;
		margin: 2rem 0;
	}

	.belief-box {
		margin: 2rem 0;
		padding: 2rem;
		background: transparent;
		border: 3px solid;
		border-image: linear-gradient(135deg, #ff5943ff, #ed8600ff) 1;
		border-radius: 0;
		text-align: center;
	}

	.belief-box p {
		font-size: 1.2rem;
		margin: 1rem 0;
	}

	.belief-box ul {
		list-style: none;
		padding: 0;
		font-size: 1.2rem;
		line-height: 2;
		margin: 1rem 0;
	}

	.belief-conclusion {
		font-size: 1.5rem !important;
		font-weight: 700;
		margin-top: 1.5rem !important;
		background: linear-gradient(90deg, #d900d5ff, #ff5943ff, #ed8600ff);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	/* CTA Section */
	.cta-section {
		text-align: center;
		margin: 4rem 0 2rem;
	}

	/* Responsive */
	@media (max-width: 768px) {
		.intro {
			flex-direction: column;
			align-items: center;
		}

		.title {
			font-size: 2.5rem;
		}

		.subtitle {
			font-size: 1.2rem;
		}

		.hero-tagline {
			font-size: 1.5rem;
		}

		.section-title {
			font-size: 2rem;
		}

		.section {
			padding: 1rem;
		}
	}
</style>
