import { readFileSync } from "node:fs";
import path from "node:path";
import { defineConfig } from "vite";
import yaml from "js-yaml";
import type { LinkItem } from "virtual:sveltepress/theme-default";
import Components from "unplugin-svelte-components/vite";
import { colors } from "./src/colors";
import { sveltepress } from "@sveltepress/vite";
import { defaultTheme } from "@sveltepress/theme-default";

const ensureLocalStorage = () => {
	if (
		typeof globalThis.localStorage !== "object" ||
		typeof globalThis.localStorage?.getItem !== "function"
	) {
		const storage = {
			length: 0,
			clear: () => {},
			getItem: () => null,
			key: () => null,
			removeItem: () => {},
			setItem: () => {},
		};

		Object.defineProperty(globalThis, "localStorage", {
			value: storage,
			configurable: true,
		});
	}
};

const navbar = yaml.load(readFileSync("./src/navbar.yaml", "utf-8"));
const sidebar = yaml.load(readFileSync("./src/sidebar.yaml", "utf-8"));

const decodeHtml = (value: string) =>
	value
		.replace(/&amp;/g, "&")
		.replace(/&lt;/g, "<")
		.replace(/&gt;/g, ">")
		.replace(/&quot;/g, '"')
		.replace(/&#39;/g, "'");

const escapeInlineCode = (input: string) => {
	let updated = false;

	const escapeSegment = (segment: string) =>
		segment.replace(/`([^`\n]*?)`/g, (match, content: string) => {
			const escaped = content
				.replace(/\\x/g, "\\\\x")
				.replace(/\\u\{/g, "\\\\u{");

			if (escaped === content) {
				return match;
			}

			updated = true;
			return `\`${escaped}\``;
		});

	const fenceRegex = /```[\s\S]*?```/g;
	let cursor = 0;
	let output = "";
	let match: RegExpExecArray | null;

	while ((match = fenceRegex.exec(input))) {
		output += escapeSegment(input.slice(cursor, match.index));
		output += match[0];
		cursor = match.index + match[0].length;
	}

	output += escapeSegment(input.slice(cursor));

	return { output, updated };
};

const config = defineConfig(() => {
	ensureLocalStorage();

	return {
		resolve: {
			alias: {
				$lib: path.resolve(__dirname, "src/lib"),
			},
		},
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
					const inlineResult = escapeInlineCode(code);
					const snippets: string[] = [];
					let index = 0;
					let replaced = false;
					let transformed = inlineResult.output.replace(
						/```fa\s*([\s\S]*?)```/g,
						(_, snippet: string) => {
							const trimmed = snippet.replace(/\s+$/, "");
							const variable = `__faCode${index}`;

							snippets.push(`const ${variable} = ${JSON.stringify(trimmed)};`);
							index += 1;
							replaced = true;

							return `<FaCode code={${variable}} />`;
						},
					);

					if (!replaced) {
						transformed = transformed.replace(
							/<pre><code class="language-fa">([\s\S]*?)<\/code><\/pre>/g,
							(_, snippet: string) => {
								const decoded = decodeHtml(snippet);
								const trimmed = decoded.replace(/\s+$/, "");
								const variable = `__faCode${index}`;

								snippets.push(
									`const ${variable} = ${JSON.stringify(trimmed)};`,
								);
								index += 1;
								replaced = true;

								return `<FaCode code={${variable}} />`;
							},
						);
					}

					if (!replaced) {
						return inlineResult.updated ? transformed : null;
					}

					const importLine = 'import FaCode from "$lib/FaCode.svelte";';
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

					return transformed;
				},
			},
			Components({
				dirs: ["src/lib"],
				extensions: ["svelte"],
				dts: "src/components.d.ts",
			}),
			sveltepress({
				theme: defaultTheme({
					navbar: navbar as LinkItem[],
					sidebar: sidebar as Record<string, LinkItem[]>,
					// github: "https://github.com/Blackman99/sveltepress",
					logo: "/fa_icon_64x64.webp",
					highlighter: {
						languages: ["ts", "svelte", "tsx", "rust"],
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
				siteConfig: {
					title: "Fa",
					description: "A programming language for the future.",
				},
			}),
		],
	};
});

export default config;
