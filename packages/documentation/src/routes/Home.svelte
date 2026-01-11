<script lang="ts">
	import EnterTheMatrix from "$lib/EnterTheMatrix.svelte";
	import FancyButton from "$lib/FancyButton.svelte";
</script>

<div style:height="4rem"></div>

<stack style:align-items="center">
	<grid class="home-grid" columns="6" grow>
		<stack
			grow
			style:grid-area="span 2 / span 4"
			style:grid-row="span 2"
			style:padding="1rem 2.5rem"
			style:justify-content="center"
			style:align-items="start"
			style:gap="0.5rem"
		>
			<div class="title">Fa</div>
			<div class="subtitle" style:font-family="'Fira Code', monospace">
				&lt;The <b>universal</b> programming language&gt;
			</div>

			<div style:padding-top="0.5rem" style:font-size="1rem">
				One coherent, safe, high-performance language — from web applications to
				tooling, systems, and beyond.
			</div>
		</stack>

		<center style:grid-area="span 2 / span 2" style:grid="span ">
			<img src="/fa_icon_500x500.webp" alt="FA Logo" width="220" />
		</center>

		<center style:grid-column="span 6" style:z-index="1">
			<FancyButton href="/getting-started/introduction">Learn Fa</FancyButton>
		</center>

		<center style:grid-column="span 6" style:font-size="21px" style:gap="8px">
			<!-- <div style:font-size="18px" style:color="var(--lightGray)">
				Meet the best of both worlds
			</div> -->
			<div class="tagline-sub">Rust with TypeScript's simplicity.</div>
			<div class="tagline-main">TypeScript at the speed of Rust.</div>
		</center>
	</grid>
</stack>

<div style:padding="8px 0">
	<EnterTheMatrix />
</div>

<stack style:align-items="center">
	<grid class="home-grid" columns="6" grow>
		<center style:grid-column="span 2">
			<div class="section-title">One language. Everywhere.</div>
		</center>
		<center>
			<div class="feature-icon">🌐</div>
			<div class="feature-title">Web applications</div>
		</center>
		<center>
			<div class="feature-icon">🛠️</div>
			<div class="feature-title">Developer tooling & CLIs</div>
		</center>
		<center>
			<div class="feature-icon">⚡</div>
			<div class="feature-title">Servers and backends</div>
		</center>
		<center>
			<div class="feature-icon">🔧</div>
			<div class="feature-title">Embedded & systems software</div>
		</center>
		<center>
			<div class="feature-icon">🎮</div>
			<div class="feature-title">Game engines and simulations</div>
		</center>
		<center>
			<div>
				Fa is built to be <strong>universal by design</strong>
			</div>
		</center>
		<center style:grid-column="span 4">
			<div style:font-size="1.15rem">
				Write once. Target multiple runtimes.<br />
				No rewrites. No paradigm switches.
			</div>
		</center>

		<!-- Typescript and Rust Section -->
		<center style:grid-column="span 4">
			<div class="section-title">
				<div>A replacement for TypeScript and Rust</div>
			</div>
		</center>
		<center style:grid-column="span 2" style:gap="0.5rem">
			<div style:font-size="1.3rem" style:font-weight="bold">Typescript</div>
			<stack style:gap="0.25rem">
				<div class="comparison-pros">✓ Productive</div>
				<div class="comparison-cons">✗ High memory usage</div>
				<div class="comparison-cons">✗ No metaprogramming</div>
			</stack>
		</center>
		<center style:grid-column="span 2" style:gap="0.5rem">
			<div style:font-size="1.3rem" style:font-weight="bold">Rust</div>
			<stack style:gap="0.25rem">
				<div class="comparison-pros">✓ Fast and safe</div>
				<div class="comparison-pros">✓ Low memory usage</div>
				<div class="comparison-cons">✗ Complex and hard to iterate</div>
			</stack>
		</center>
		<center style:grid-column="span 4" style:gap="0.5rem">
			<div
				style:font-size="1.5rem"
				style:font-weight="bold"
				style:color="var(--purpleLight)"
			>
				Meet the best of both worlds
			</div>
			<stack style:gap="0.25rem">
				<div>The <strong>ergonomics of TypeScript</strong></div>
				<div>The <strong>performance and safety of Rust</strong></div>
				<div>Without the downsides of either</div>
			</stack>
		</center>

		<!-- Performance Section -->
		<center style:grid-area="span 2 / span 1">
			<h2 class="section-title">Performance without compromise</h2>
		</center>
		<center style:grid-column="span 3">
			<div style:font-size="1.05rem">
				Fa compiles to efficient low-level code while remaining simple to reason
				about.
			</div>
		</center>
		<center style:grid-column="span 3">
			<stack style:align-items="start" style:gap="0.5rem">
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
			</stack>
		</center>

		<!-- Memory Section -->
		<center style:grid-column="span 4">
			<stack style:gap="1rem">
				<div class="section-title" style:padding="0">
					A new memory model — simple, safe, powerful
				</div>
				<div>Fa introduces a unique memory management system</div>
			</stack>
		</center>
		<center style:grid-column="span 4">
			<stack style:gap="1rem">
				<div>❌ No garbage collector</div>
				<div>❌ No complex borrow-checker rules</div>
				<div>❌ No lifetime annotations everywhere</div>
			</stack>
		</center>
		<center style:grid-area="span 2 / span 1">
			<div style:font-weight="bold" style:font-size="1.15rem">
				Instead, Fa relies on two core concepts
			</div>
		</center>
		<center
			style:grid-column="span 2"
			style:align-items="start"
			style:padding-right="0.5rem"
		>
			<stack
				style:align-items="start"
				style:gap="0.5rem"
				style:text-align="start"
			>
				<div
					style:color="var(--orangeLight)"
					style:font-weight="bold"
					style:font-size="1.5rem"
				>
					Owned References
				</div>
				<div style:font-size="1.05rem" style:text-wrap="wrap">
					Ownership integrated into the type-system
				</div>
			</stack>
		</center>
		<center style:grid-area="span 2 / span 1">
			<div style:z-index="0">
				<FancyButton href="/">Learn more</FancyButton>
			</div>
		</center>
		<center style:grid-column="span 2" style:align-items="start">
			<stack
				style:align-items="start"
				style:gap="0.5rem"
				style:text-align="start"
			>
				<div
					style:color="var(--orangeLight)"
					style:font-weight="bold"
					style:font-size="1.5rem"
				>
					Relations
				</div>
				<div>Declare relationships between objects</div>
			</stack>
		</center>

		<!-- Reactivity Section -->
		<center style:grid-column="span 4">
			<stack style:gap="1rem">
				<div class="section-title" style:padding="0">
					Built-in reactivity — at the language level
				</div>
				<div style:font-weight="bold">
					Fa is the first programming language with reactivity built into its
					core semantics.
				</div>
			</stack>
		</center>
		<center
			style:font-weight="bold"
			style:color="var(--purpleLight)"
			style:font-size="1.1rem"
		>
			Reactivity in Fa is...</center
		>
		<center>
			<div class="feature-icon">⚙️</div>
			<div class="feature-title">Compiled</div>
		</center>
		<center>
			<div class="feature-icon">🛡️</div>
			<div class="feature-title">Type-safe</div>
		</center>
		<center>
			<div class="feature-icon">🎯</div>
			<div class="feature-title">Predictable</div>
		</center>
	</grid>
</stack>

<div style:padding="16px 0">
	<EnterTheMatrix />
</div>

<stack style:align-items="center">
	<grid class="home-grid" columns="6" grow>
		<!-- Metaprogramming Section -->
		<center>
			<div>If you can write Fa, you can metaprogram in Fa.</div>
		</center>
		<center style:grid-column="span 3">
			<stack style:gap="1rem">
				<div class="section-title" style:padding="0">
					Metaprogramming, without pain
				</div>
				<div style:font-weight="bold" style:font-size="1.1rem">
					Fa makes metaprogramming <strong>boringly simple.</strong>
				</div>
			</stack>
		</center>
		<center>
			<div class="feature-icon">⚡</div>
			<div class="feature-title">Compile-time execution</div>
		</center>
		<center>
			<div class="feature-icon">🔨</div>
			<div class="feature-title">Code generation</div>
		</center>
		<center>
			<div class="feature-icon">📝</div>
			<div class="feature-title">No separate DSLs</div>
		</center>
		<center>
			<div class="feature-icon">😵‍💫</div>
			<div class="feature-title">No unreadable syntax trees</div>
		</center>

		<!-- Designed for the Future -->
		<center style:grid-column="span 4">
			<stack style:gap="1rem">
				<div class="section-title">
					Designed for humans and AIs. Built for the future.
				</div>
			</stack>
		</center>
		<center>Strongly typed</center>
		<center>Fully open source</center>
		<center>Vendor-neutral</center>
		<center>Designed for long-term maintainability</center>

		<center style:grid-column="span 4">
			<stack style:font-size="1.2rem" style:gap="1rem">
				<div>This is not an experiment</div>
				<div class="foundation-emphasis">It's a foundation</div>
			</stack>
		</center>

		<center style:grid-column="span 4">
			<div style:z-index="0">
				<FancyButton href="/getting-started/introduction">Learn Fa</FancyButton>
			</div>
		</center>
	</grid>
</stack>

<div style:height="15dvh"></div>

<style>
	.home-grid {
		grid-template-columns: repeat(4, 1fr);
		flex-grow: 1;
		width: 900px;
		max-width: 100%;
		gap: 1px;
		background-color: var(--lightGray);
		border: 1px solid var(--lightGray);
		grid-auto-rows: 180px;
	}

	.home-grid > * {
		background-color: var(--white);
		padding: 2rem;
		text-wrap: balance;
	}

	.title {
		font-size: 3.75rem;
		font-family: "Audiowide", cursive;
		font-weight: 900;
		font-optical-sizing: auto;
		letter-spacing: 0.02em;
		font-style: normal;
		color: var(--white);
		/*background: linear-gradient(45deg, #d900d5, #ff5943, #ed8600);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;*/
		position: relative;
		z-index: 0;
		margin-left: 16px;
		padding: 0 4px;

		&::before {
			content: "";
			position: absolute;
			inset: 6px -12px;
			transform: skew(-16deg);
			z-index: -1;
			background: linear-gradient(
				45deg,
				var(--purpleLight),
				var(--orangeDark),
				var(--orangeLight),
				var(--purpleLight)
			);
		}
	}

	.subtitle {
		font-size: 1.5rem;
	}

	.tagline-main {
		font-weight: 700;
		background: linear-gradient(135deg, var(--orangeDark), var(--orangeLight));
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.tagline-sub {
		font-weight: 700;
		background: linear-gradient(135deg, var(--purpleLight), var(--redDark));
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.section-title {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 1.4rem;
		font-weight: 700;
		text-align: center;
		background: linear-gradient(
			90deg,
			var(--purpleLight),
			var(--orangeDark),
			var(--orangeLight)
		);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.feature-icon {
		font-size: 2.1rem;
		margin-bottom: 0.5rem;
	}

	.feature-title {
		font-size: 1.1rem;
		font-weight: 600;
	}

	.comparison-pros {
		color: var(--green);
		font-size: 1.1rem;
		font-weight: 500;
	}

	.comparison-cons {
		color: var(--redLight);
		font-size: 1.1rem;
		font-weight: 500;
	}

	.benefit-item {
		display: flex;
		align-items: center;
		gap: 1rem;
		font-size: 1.05rem;
	}

	.benefit-icon {
		font-size: 1.4rem;
	}

	.foundation-emphasis {
		font-weight: 700;
		font-size: 2rem;
		background: linear-gradient(10deg, var(--purpleLight), var(--purpleLight));
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	/* Responsive */
	@media (max-width: 768px) {
		.title {
			font-size: 2.5rem;
		}

		.subtitle {
			font-size: 1.2rem;
		}

		.section-title {
			font-size: 2rem;
		}
	}
</style>
