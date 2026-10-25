import {
	type BundledLanguage,
	createHighlighter,
	type LanguageRegistration,
	type ThemeRegistration,
} from "shiki";

import faGrammar from "./fa.tmLanguage.json" with { type: "json" };

/** The languages used in the documentation, besides Fa. */
const LANGUAGES: BundledLanguage[] = [
	"bash",
	"c",
	"csharp",
	"go",
	"java",
	"javascript",
	"json",
	"python",
	"rust",
	"typescript",
	"yaml",
];

/**
 * A theme whose colors are CSS variables, so that code follows the design system
 * and switches between the light and dark themes without highlighting twice.
 */
const theme: ThemeRegistration = {
	name: "fa",
	type: "dark",
	colors: {
		"editor.background": "var(--code-background)",
		"editor.foreground": "var(--code-text)",
	},
	tokenColors: [
		{ scope: ["comment"], settings: { foreground: "var(--code-comment)", fontStyle: "italic" } },
		{ scope: ["string"], settings: { foreground: "var(--code-string)" } },
		{
			scope: ["constant.character.escape", "punctuation.definition.interpolation"],
			settings: { foreground: "var(--code-escape)" },
		},
		{ scope: ["meta.interpolation"], settings: { foreground: "var(--code-text)" } },
		{
			scope: ["constant.numeric", "constant.language", "variable.language"],
			settings: { foreground: "var(--code-constant)" },
		},
		{
			scope: ["storage", "storage.type", "storage.modifier"],
			settings: { foreground: "var(--code-declaration)" },
		},
		{
			scope: ["keyword", "keyword.control"],
			settings: { foreground: "var(--code-keyword)" },
		},
		{
			scope: ["keyword.operator", "punctuation"],
			settings: { foreground: "var(--code-operator)" },
		},
		{
			scope: ["keyword.operator.word"],
			settings: { foreground: "var(--code-keyword)" },
		},
		{
			scope: ["keyword.other.effect"],
			settings: { foreground: "var(--code-effect)", fontStyle: "bold" },
		},
		{
			scope: ["entity.name.function", "support.function", "meta.function-call"],
			settings: { foreground: "var(--code-function)" },
		},
		{
			scope: ["entity.name.function.macro"],
			settings: { foreground: "var(--code-macro)" },
		},
		{
			scope: ["entity.name.type", "support.type", "entity.name.class", "storage.type.primitive"],
			settings: { foreground: "var(--code-type)" },
		},
		{ scope: ["variable.parameter"], settings: { foreground: "var(--code-text)" } },
	],
};

let highlighter: ReturnType<typeof createHighlighter> | undefined;

export function getHighlighter() {
	highlighter ??= createHighlighter({
		themes: [theme],
		langs: [...LANGUAGES, faGrammar as unknown as LanguageRegistration],
	});
	return highlighter;
}

export const THEME = "fa";

/**
 * Highlights a snippet of code into HTML (a `<pre>` element).
 * `errorLine` (starting at 1) marks a line as erroneous, to underline it.
 */
export async function highlight(
	code: string,
	options: { language?: string; errorLine?: number } = {},
) {
	const instance = await getHighlighter();
	const language = options.language ?? "fa";
	const lang = instance.getLoadedLanguages().includes(language) ? language : "text";
	return instance.codeToHtml(code.replace(/\n$/, ""), {
		lang,
		theme: THEME,
		transformers: [
			{
				line(node, line) {
					if (line === options.errorLine) {
						this.addClassToHast(node, "error-line");
					}
				},
			},
		],
	});
}

/** The colored tokens of a single line of Fa code. */
export async function tokenize(line: string) {
	const instance = await getHighlighter();
	// Fa is registered from its grammar, so it is not one of Shiki's bundled languages
	const { tokens } = instance.codeToTokens(line, { lang: "fa" as BundledLanguage, theme: THEME });
	return (tokens[0] ?? []).map((token) => ({
		content: token.content,
		offset: token.offset,
		color: token.color ?? "var(--code-text)",
		bold: ((token.fontStyle ?? 0) & 2) !== 0,
	}));
}
