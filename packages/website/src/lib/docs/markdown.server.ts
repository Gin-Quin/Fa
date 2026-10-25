import GithubSlugger from "github-slugger";
import type { Element, ElementContent, Root as HastRoot } from "hast";
import { toString as hastToString } from "hast-util-to-string";
import type { Root as MdastRoot } from "mdast";
import rehypeStringify from "rehype-stringify";
import remarkDirective from "remark-directive";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { unified } from "unified";
import { SKIP, visit } from "unist-util-visit";

import { getHighlighter, THEME } from "#lib/highlight/highlighter.server.ts";

import type { PageSection, TocEntry } from "./types.ts";

export type RenderedMarkdown = {
	title: string;
	description: string;
	html: string;
	toc: TocEntry[];
	sections: PageSection[];
};

const CALLOUTS: Record<string, string> = {
	note: "Note",
	tip: "Tip",
	info: "Info",
	caution: "Caution",
	warning: "Warning",
};

const LANGUAGE_NAMES: Record<string, string> = {
	fa: "Fa",
	bash: "Terminal",
	sh: "Terminal",
	c: "C",
	cpp: "C++",
	csharp: "C#",
	go: "Go",
	java: "Java",
	js: "JavaScript",
	javascript: "JavaScript",
	json: "JSON",
	python: "Python",
	rs: "Rust",
	rust: "Rust",
	ts: "TypeScript",
	typescript: "TypeScript",
	yaml: "YAML",
};

const LANGUAGE_ALIASES: Record<string, string> = {
	js: "javascript",
	ts: "typescript",
	rs: "rust",
	sh: "bash",
};

/**
 * Turns `:::tip` … `:::` containers into callouts, and puts back as text the
 * `:name` and `::name` directives, which are not used by the documentation
 * (a `:` followed by a word is plain text, like in `12:30`).
 */
function remarkCallouts(source: string) {
	return () => (tree: MdastRoot) => {
		visit(tree, (node, index, parent) => {
			if (node.type === "containerDirective") {
				const kind = node.name in CALLOUTS ? node.name : "note";
				node.data = {
					hName: "aside",
					hProperties: { className: ["callout", `callout-${kind}`] },
				};
				node.children.unshift({
					type: "paragraph",
					data: { hName: "p", hProperties: { className: ["callout-title"] } },
					children: [{ type: "text", value: CALLOUTS[kind] ?? "Note" }],
				});
				return;
			}
			if (
				(node.type === "textDirective" || node.type === "leafDirective") &&
				parent &&
				index !== undefined
			) {
				const start = node.position?.start.offset ?? 0;
				const end = node.position?.end.offset ?? start;
				const text = { type: "text" as const, value: source.slice(start, end) };
				parent.children.splice(
					index,
					1,
					node.type === "leafDirective"
						? ({ type: "paragraph", children: [text] } as never)
						: (text as never),
				);
				return [SKIP, index];
			}
		});
	};
}

/**
 * Gives GitHub-style ids to the headings (the ones used by the links of the documentation),
 * adds an anchor link to each, and collects the table of contents and the search sections.
 */
function rehypeHeadings(result: { title: string; toc: TocEntry[]; sections: PageSection[] }) {
	return () => (tree: HastRoot) => {
		const slugger = new GithubSlugger();
		let current: PageSection = { id: "", heading: "", text: "" };
		result.sections.push(current);

		const collectText = (node: ElementContent) => {
			const text = hastToString(node).trim();
			if (text) {
				current.text += `${current.text ? " " : ""}${text}`;
			}
		};

		for (const node of tree.children) {
			if (node.type !== "element") {
				continue;
			}
			const depth = /^h([1-6])$/.exec(node.tagName)?.[1];
			if (!depth) {
				collectText(node);
				continue;
			}
			const text = hastToString(node).trim();
			const id = slugger.slug(text);
			node.properties.id = id;

			if (depth === "1") {
				result.title ||= text;
				continue;
			}

			node.children = [
				{
					type: "element",
					tagName: "a",
					properties: { href: `#${id}`, className: ["heading-anchor"] },
					children: node.children,
				},
			];

			if (depth === "2" || depth === "3") {
				result.toc.push({ id, text, depth: Number(depth) as 2 | 3 });
			}
			current = { id, heading: text, text: "" };
			result.sections.push(current);
		}
	};
}

/** Highlights the code blocks with Shiki, and wraps them in a frame with their language and a copy button. */
function rehypeCode() {
	return async (tree: HastRoot) => {
		const highlighter = await getHighlighter();
		const blocks: Array<{ parent: HastRoot | Element; index: number; pre: Element }> = [];

		visit(tree, "element", (node, index, parent) => {
			if (node.tagName === "pre" && parent && index !== undefined) {
				blocks.push({ parent, index, pre: node });
				return SKIP;
			}
		});

		for (const { parent, index, pre } of blocks) {
			const code = pre.children.find(
				(child): child is Element => child.type === "element" && child.tagName === "code",
			);
			if (!code) {
				continue;
			}
			const classNames = (code.properties.className as string[] | undefined) ?? [];
			const written = classNames.find((name) => name.startsWith("language-"))?.slice(9) ?? "";
			const language = LANGUAGE_ALIASES[written] ?? written;
			const loaded = highlighter.getLoadedLanguages().includes(language);
			const source = hastToString(code).replace(/\n$/, "");

			const highlighted = highlighter.codeToHast(source, {
				lang: loaded ? language : "text",
				theme: THEME,
			});
			const highlightedPre = highlighted.children[0] as Element;
			highlightedPre.properties.tabIndex = 0;

			const label = LANGUAGE_NAMES[written] ?? (written || "Text");

			parent.children[index] = {
				type: "element",
				tagName: "figure",
				properties: { className: ["code-block"], dataLanguage: written || "text" },
				children: [
					{
						type: "element",
						tagName: "figcaption",
						properties: { className: ["code-block-header"] },
						children: [
							{
								type: "element",
								tagName: "span",
								properties: { className: ["code-block-language"] },
								children: [{ type: "text", value: label }],
							},
							{
								type: "element",
								tagName: "button",
								properties: {
									type: "button",
									className: ["code-block-copy"],
									ariaLabel: "Copy the code",
								},
								children: [{ type: "text", value: "Copy" }],
							},
						],
					},
					highlightedPre,
				],
			};
		}
	};
}

/** Wraps the tables so that they scroll horizontally on small screens, and opens external links in a new tab. */
function rehypeLayout() {
	return (tree: HastRoot) => {
		visit(tree, "element", (node, index, parent) => {
			if (node.tagName === "table" && parent && index !== undefined) {
				parent.children[index] = {
					type: "element",
					tagName: "div",
					properties: { className: ["table-wrapper"] },
					children: [node],
				};
				return SKIP;
			}
			if (node.tagName === "a") {
				const href = String(node.properties.href ?? "");
				if (/^https?:\/\//.test(href)) {
					node.properties.target = "_blank";
					node.properties.rel = ["noopener", "noreferrer"];
				}
			}
		});
	};
}

export async function renderMarkdown(source: string): Promise<RenderedMarkdown> {
	const result = { title: "", toc: [] as TocEntry[], sections: [] as PageSection[] };

	const file = await unified()
		.use(remarkParse)
		.use(remarkGfm)
		.use(remarkDirective)
		.use(remarkCallouts(source))
		.use(remarkRehype)
		.use(rehypeHeadings(result))
		.use(rehypeCode)
		.use(rehypeLayout)
		.use(rehypeStringify)
		.process(source);

	// the title is displayed by the page layout, not by the Markdown
	const html = String(file).replace(/^\s*<h1[^>]*>[\s\S]*?<\/h1>\s*/, "");
	const introduction = result.sections[0]?.text ?? "";

	return {
		title: result.title,
		description: introduction.length > 180 ? `${introduction.slice(0, 177)}…` : introduction,
		html,
		toc: result.toc,
		sections: result.sections.filter((section) => section.text || section.heading),
	};
}
