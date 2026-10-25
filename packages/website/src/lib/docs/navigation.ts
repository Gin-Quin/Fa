/**
 * The order of the documentation pages in the sidebar.
 * Every Markdown file of the `documentation/` folder must be listed here: the build fails otherwise.
 * Page titles come from the first heading of each Markdown file.
 */
export const navigation: Array<{ title: string; pages: string[] }> = [
	{
		title: "Getting started",
		pages: [
			"getting-started/introduction",
			"getting-started/installation",
			"getting-started/syntax-basics",
		],
	},
	{
		title: "Values",
		pages: [
			"values/declaring-values",
			"values/numbers",
			"values/booleans",
			"values/strings",
			"values/objects",
			"values/tuples",
			"values/optionals",
			"values/ranges",
			"values/errors",
		],
	},
	{
		title: "Operations",
		pages: [
			"operations/arithmetic",
			"operations/logical",
			"operations/string",
			"operations/destructuring",
		],
	},
	{
		title: "Control flow",
		pages: ["control-flow/conditionals", "control-flow/loops", "control-flow/do"],
	},
	{
		title: "Collections",
		pages: [
			"collections/arrays",
			"collections/sets",
			"collections/maps",
			"collections/bags",
			"collections/columnars",
			"collections/growing-collections",
		],
	},
	{
		title: "Functions",
		pages: ["functions/defining-functions", "functions/function-composition"],
	},
	{
		title: "Types",
		pages: [
			"types/declaring-types",
			"types/generics",
			"types/enumerations",
			"types/fields",
			"types/unions",
			"types/operations",
			"types/type-functions",
			"types/type-conversion",
			"types/types-at-runtime",
		],
	},
	{
		title: "Project structure",
		pages: [
			"architecture/overview",
			"architecture/modules",
			"architecture/packages",
			"architecture/workspaces",
			"architecture/configuration",
			"architecture/dependencies",
			"architecture/testing",
			"architecture/hot-reloading",
		],
	},
	{
		title: "Advanced",
		pages: [
			"advanced/memory-management",
			"advanced/owned-references",
			"advanced/relations",
			"advanced/requirements",
			"advanced/reactivity",
			"advanced/concurrency",
			"advanced/metaprogramming",
			"advanced/structural-typing",
		],
	},
	{
		title: "Guides",
		pages: [
			"guides/hello-world",
			"guides/build-your-application",
			"guides/performance",
			"guides/web-server",
			"guides/deploy-a-library",
		],
	},
	{
		title: "Reference",
		pages: ["reference/language-reference"],
	},
	{
		title: "Fa for…",
		pages: [
			"fa-for/fa-for-typescript-developers",
			"fa-for/fa-for-rust-developers",
			"fa-for/fa-for-go-developers",
			"fa-for/fa-for-python-developers",
			"fa-for/fa-for-c-developers",
			"fa-for/fa-for-java-developers",
			"fa-for/fa-for-csharp-developers",
		],
	},
];
