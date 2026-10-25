<script lang="ts">
	import GithubIcon from "./GithubIcon.svelte";
	import Logo from "./Logo.svelte";

	const COLUMNS = [
		{
			title: "Learn",
			links: [
				{ label: "Introduction", href: "/getting-started/introduction" },
				{ label: "Syntax basics", href: "/getting-started/syntax-basics" },
				{ label: "Language reference", href: "/reference/language-reference" },
				{ label: "Hello World", href: "/guides/hello-world" },
			],
		},
		{
			title: "Concepts",
			links: [
				{ label: "Typed errors", href: "/values/errors" },
				{ label: "Requirements", href: "/advanced/requirements" },
				{ label: "Owned references", href: "/advanced/owned-references" },
				{ label: "Relations", href: "/advanced/relations" },
			],
		},
		{
			title: "Coming from",
			links: [
				{ label: "TypeScript", href: "/fa-for/fa-for-typescript-developers" },
				{ label: "Rust", href: "/fa-for/fa-for-rust-developers" },
				{ label: "Go", href: "/fa-for/fa-for-go-developers" },
				{ label: "Python", href: "/fa-for/fa-for-python-developers" },
			],
		},
	];
</script>

<footer>
	<div class="container inner">
		<div class="brand">
			<Logo size={32} />
			<p>A programming language for AI agents and humans.</p>
			<a
				class="github"
				href="https://github.com/Gin-Quin/Fa"
				target="_blank"
				rel="noopener noreferrer"
			>
				<GithubIcon size={16} /> Gin-Quin/Fa
			</a>
		</div>

		{#each COLUMNS as column (column.title)}
			<div class="column">
				<p class="title">{column.title}</p>
				<ul>
					{#each column.links as link (link.href)}
						<li><a href={link.href}>{link.label}</a></li>
					{/each}
				</ul>
			</div>
		{/each}
	</div>
	<div class="container bottom">
		<span>MIT License</span>
		<span>Fa is in active development: expect breaking changes.</span>
	</div>
</footer>

<style>
	footer {
		position: relative;
		border-top: 1px solid var(--color-border);
		background: var(--color-background-elevated);
	}

	.inner {
		display: grid;
		grid-template-columns: 2fr repeat(3, 1fr);
		gap: var(--space-7);
		padding-block: var(--space-8) var(--space-7);
	}

	.brand {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-4);
	}

	.brand p {
		max-width: 22rem;
		color: var(--color-text-muted);
		font-size: var(--text-sm);
	}

	.github {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
		color: var(--color-text-muted);
	}

	.github:hover {
		color: var(--color-text);
	}

	.title {
		margin-bottom: var(--space-4);
		font-size: var(--text-xs);
		font-weight: var(--weight-semibold);
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
		color: var(--color-text-subtle);
	}

	ul {
		list-style: none;
		padding: 0;
		display: grid;
		gap: var(--space-3);
	}

	.column a {
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		transition: color var(--duration-fast);
	}

	.column a:hover {
		color: var(--color-text);
	}

	.bottom {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: var(--space-3);
		padding-block: var(--space-5);
		border-top: 1px solid var(--color-border);
		font-size: var(--text-xs);
		color: var(--color-text-subtle);
	}

	@media (max-width: 820px) {
		.inner {
			grid-template-columns: 1fr 1fr;
		}

		.brand {
			grid-column: 1 / -1;
		}
	}
</style>
