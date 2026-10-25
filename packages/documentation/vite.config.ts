import { readFileSync } from "node:fs";
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";
import { load } from "js-yaml";
import type { LinkItem } from "virtual:sveltepress/theme-default";
import { colors } from "./src/colors.ts";
import { escapeSvelteSyntax } from "./src/markdown/escapeSvelteSyntax.ts";
import { githubHeadingIds } from "./src/markdown/githubHeadingIds.ts";
import faGrammar from "./src/shiki/fa.tmLanguage.json" with { type: "json" };
import { sveltepress } from "@sveltepress/vite";
import { defaultTheme } from "@sveltepress/theme-default";

const navbar = load(readFileSync("./src/navbar.yaml", "utf-8"));
const sidebar = load(readFileSync("./src/sidebar.yaml", "utf-8"));

const decodeHtml = (value: string) =>
	value
		.replace(/&amp;/g, "&")
		.replace(/&lt;/g, "<")
		.replace(/&gt;/g, ">")
		.replace(/&quot;/g, '"')
		.replace(/&#39;/g, "'");

const config = defineConfig(() => {
	return {
		server: {
			port: 8000,
		},
		plugins: [
			{
				name: "fa-code-fences",
				enforce: "pre",
				transform(code: string, id: string) {
					if (!id.endsWith(".md")) {
						return null;
					}
					const snippets: string[] = [];
					let index = 0;
					let replaced = false;
					let transformed = code.replace(
						/```(fa|ts|typescript|rust|bash|sh|js|rs)\s*([\s\S]*?)```/g,
						(_, lang: string, snippet: string) => {
							const trimmed = snippet.replace(/\s+$/, "");
							const variable = `__code${index}`;

							snippets.push(`const ${variable} = ${JSON.stringify(trimmed)};`);
							index += 1;
							replaced = true;

							return `<Code code={${variable}} lang="${lang}" />`;
						},
					);

					if (!replaced) {
						transformed = transformed.replace(
							/<pre><code class="language-(fa|ts|typescript|rust|bash|sh|js|rs)">([\s\S]*?)<\/code><\/pre>/g,
							(_, lang: string, snippet: string) => {
								const decoded = decodeHtml(snippet);
								const trimmed = decoded.replace(/\s+$/, "");
								const variable = `__code${index}`;

								snippets.push(
									`const ${variable} = ${JSON.stringify(trimmed)};`,
								);
								index += 1;
								replaced = true;

								return `<Code code={${variable}} lang="${lang}" />`;
							},
						);
					}

					if (!replaced) {
						return null;
					}

					const importLine = 'import Code from "#lib/Code.svelte";';
					const scriptOpenMatch = transformed.match(/<script\b[^>]*>/);
					const scriptCloseMatch = transformed.match(/<\/script>/);

					if (scriptOpenMatch?.index && scriptCloseMatch) {
						if (!transformed.includes(importLine)) {
							const [tag] = scriptOpenMatch;
							const insertAt = scriptOpenMatch.index + tag.length;
							transformed =
								transformed.slice(0, insertAt) +
								`\n\t${importLine}` +
								transformed.slice(insertAt);
						}

						const insertAt = scriptCloseMatch.index ?? transformed.length;
						const insertBlock = `\n\t${snippets.join("\n\t")}\n`;

						transformed =
							transformed.slice(0, insertAt) +
							insertBlock +
							transformed.slice(insertAt);
					} else {
						const scriptBlock = `<script>\n\t${importLine}\n\t${snippets.join("\n\t")}\n</script>\n`;

						transformed = scriptBlock + transformed;
					}

					// The Markdown pages have no meaningful source map.
					return { code: transformed, map: { mappings: "" } };
				},
			},
			sveltepress({
				theme: defaultTheme({
					navbar: navbar as LinkItem[],
					sidebar: sidebar as Record<string, LinkItem[]>,
					github: "https://github.com/Gin-Quin/Fa",
					logo: "/fa_icon_64x64.webp",
					highlighter: {
						languages: [
							"ts",
							"js",
							"svelte",
							"tsx",
							"rust",
							"bash",
							"sh",
							"c",
							"csharp",
							"go",
							"java",
							"python",
							"yaml",
							faGrammar as any,
						],
					},
					themeColor: {
						dark: colors.orangeDark,
						light: colors.orangeLight,
						hover: colors.redDark,
						primary: colors.orangeLight,
						gradient: {
							start: colors.orangeLight,
							end: colors.purpleLight,
						},
					},
				}),
				remarkPlugins: [githubHeadingIds, escapeSvelteSyntax],
				svelteKitOptions: {
					adapter: adapter(),
					preprocess: [vitePreprocess()],
					compilerOptions: {
						runes: true,
					},
				},
				siteConfig: {
					title: "Fa, The Programming Language",
					pageTitle: "Fa",
					description: "A programming language for the future.",
				},
			}),
		],
	};
});

export default config;
