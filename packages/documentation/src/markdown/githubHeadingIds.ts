/**
 * Remark plugin that gives headings the same ids as on GitHub.
 *
 * The documentation links to sections with GitHub-style fragments
 * (`/operations/logical#no-truthiness`), so that the links also work when the
 * Markdown is read on GitHub. The Sveltepress theme generates other ids
 * (`No-truthiness`), so this plugin, which must run after the theme's
 * `anchors` plugin, rewrites them: the heading ids, the heading links and the
 * table of contents.
 */

type MarkdownNode = {
	type: string;
	children?: MarkdownNode[];
	data?: {
		hProperties?: Record<string, unknown>;
	};
};

type Anchor = {
	slugId: string;
	title: string;
	depth: number;
};

type MarkdownFile = {
	/** Filled by the theme's `anchors` plugin with `{ anchors: Anchor[] }`. */
	data: object;
};

/**
 * Like `github-slugger`: lowercase, drop the punctuation, and
 * replace the spaces with dashes.
 */
const slugify = (title: string) =>
	title
		.toLowerCase()
		.replace(/[^\p{L}\p{M}\p{N}\p{Pc} -]/gu, "")
		.replace(/ /g, "-");

const visit = (node: MarkdownNode, callback: (node: MarkdownNode) => void) => {
	callback(node);
	for (const child of node.children ?? []) {
		visit(child, callback);
	}
};

export const githubHeadingIds = () => (tree: MarkdownNode, file: MarkdownFile) => {
	const anchors = (file.data as { anchors?: Anchor[] }).anchors ?? [];
	const ids = new Map<string, string>();
	const occurrences = new Map<string, number>();

	for (const anchor of anchors) {
		const slug = slugify(anchor.title);
		const count = occurrences.get(slug) ?? 0;
		occurrences.set(slug, count + 1);
		const id = count === 0 ? slug : `${slug}-${count}`;
		ids.set(anchor.slugId, id);
		anchor.slugId = id;
	}

	visit(tree, (node) => {
		const properties = node.data?.hProperties;
		if (!properties) {
			return;
		}
		if (node.type === "AnchorId" && typeof properties.id === "string") {
			properties.id = ids.get(properties.id) ?? properties.id;
		} else if (node.type === "Anchor" && typeof properties.href === "string") {
			const id = ids.get(properties.href.slice(1));
			if (id) {
				properties.href = `#${id}`;
			}
		}
	});
};
