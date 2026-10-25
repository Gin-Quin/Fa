/**
 * Remark plugin that makes Markdown text and inline code literal for Svelte.
 *
 * Sveltepress compiles each Markdown page into a Svelte component, so a `{`
 * in the text is read as a Svelte expression, and a `${` in inline code is
 * read as a JavaScript interpolation (Sveltepress renders inline code as a
 * template literal). The documentation is plain Markdown that talks about
 * code full of braces, so this plugin turns every text node and inline code
 * node that could be misread into an HTML node whose braces are written as
 * `&#123;` and `&#125;`.
 *
 * Raw HTML in the Markdown (including the `<Code />` components generated for
 * the code fences) is left untouched and keeps its Svelte meaning.
 */

type MarkdownNode = {
	type: string;
	value?: string;
	children?: MarkdownNode[];
	position?: {
		start: { offset?: number };
		end: { offset?: number };
	};
};

type MarkdownFile = {
	value: unknown;
};

const escapeHtml = (value: string) =>
	value
		.replace(/&/g, "&amp;")
		.replace(/</g, "&lt;")
		.replace(/>/g, "&gt;")
		.replace(/\{/g, "&#123;")
		.replace(/\}/g, "&#125;");

const hasBraces = (value: string) => value.includes("{") || value.includes("}");

const toHtmlNode = (html: string): MarkdownNode => ({ type: "html", value: html });

/**
 * Returns the literal HTML a node must be replaced with, or `undefined` to keep it.
 */
const escapeNode = (node: MarkdownNode, source: string): MarkdownNode | undefined => {
	switch (node.type) {
		case "inlineCode":
			return toHtmlNode(`<code>${escapeHtml(node.value ?? "")}</code>`);

		case "text":
			return node.value && hasBraces(node.value) ? toHtmlNode(escapeHtml(node.value)) : undefined;

		// `remark-directive` reads `:name{...}` in the text as a directive, that
		// Sveltepress turns back into raw text after this plugin has run.
		case "textDirective":
		case "leafDirective": {
			const start = node.position?.start.offset;
			const end = node.position?.end.offset;
			if (start === undefined || end === undefined) {
				return undefined;
			}
			const raw = source.slice(start, end);
			return hasBraces(raw) ? toHtmlNode(escapeHtml(raw)) : undefined;
		}

		default:
			return undefined;
	}
};

const visit = (node: MarkdownNode, source: string) => {
	if (!node.children) {
		return;
	}
	node.children = node.children.map((child) => {
		const replacement = escapeNode(child, source);
		if (replacement) {
			return replacement;
		}
		visit(child, source);
		return child;
	});
};

export const escapeSvelteSyntax = () => (tree: MarkdownNode, file: MarkdownFile) => {
	visit(tree, String(file.value));
};
