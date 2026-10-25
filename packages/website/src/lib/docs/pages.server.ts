import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";

import { type RenderedMarkdown, renderMarkdown } from "./markdown.server.ts";
import { navigation } from "./navigation.ts";
import type { NavigationSection, PageLink } from "./types.ts";

/** The folder holding the Markdown sources: the website is built from `packages/website`. */
export const DOCUMENTATION_FOLDER = resolve(process.cwd(), "../../documentation");

export const REPOSITORY_URL = "https://github.com/Gin-Quin/Fa";

export type DocumentationPage = RenderedMarkdown & {
	slug: string;
	section: string;
	editUrl: string;
};

const ORDER = navigation.flatMap((section) => section.pages);

function listMarkdownFiles(folder: string): string[] {
	return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
		const path = join(folder, entry.name);
		if (entry.isDirectory()) {
			return listMarkdownFiles(path);
		}
		return entry.name.endsWith(".md") ? [path] : [];
	});
}

/** Fails when a Markdown file is missing from the navigation, or the other way around. */
function checkNavigation() {
	const files = listMarkdownFiles(DOCUMENTATION_FOLDER)
		.map((path) => relative(DOCUMENTATION_FOLDER, path).replace(/\.md$/, ""))
		// the README describes the folder, it is not a page
		.filter((slug) => slug !== "README");

	const unlisted = files.filter((slug) => !ORDER.includes(slug));
	const missing = ORDER.filter((slug) => !files.includes(slug));
	const problems = [
		unlisted.length > 0 &&
			`These pages are not listed in src/lib/docs/navigation.ts: ${unlisted.join(", ")}`,
		missing.length > 0 && `These pages of the navigation do not exist: ${missing.join(", ")}`,
	].filter(Boolean);

	if (problems.length > 0) {
		throw new Error(problems.join("\n"));
	}
}

checkNavigation();

const pathOf = (slug: string) => join(DOCUMENTATION_FOLDER, `${slug}.md`);

const cache = new Map<string, { modified: number; page: Promise<DocumentationPage> }>();

/** Renders a page, once per version of its file. */
export function getPage(slug: string): Promise<DocumentationPage> | undefined {
	if (!ORDER.includes(slug)) {
		return undefined;
	}
	const modified = statSync(pathOf(slug)).mtimeMs;
	const cached = cache.get(slug);
	if (cached?.modified === modified) {
		return cached.page;
	}
	const page = renderMarkdown(readFileSync(pathOf(slug), "utf-8")).then((rendered) => ({
		...rendered,
		slug,
		section: navigation.find((section) => section.pages.includes(slug))?.title ?? "",
		editUrl: `${REPOSITORY_URL}/edit/main/documentation/${slug}.md`,
	}));
	cache.set(slug, { modified, page });
	return page;
}

export function getAllPages() {
	return Promise.all(ORDER.map((slug) => getPage(slug) as Promise<DocumentationPage>));
}

/** The title of a page is the first heading of its Markdown file. */
function readTitle(slug: string) {
	const source = readFileSync(pathOf(slug), "utf-8");
	return /^#\s+(.+)$/m.exec(source)?.[1].trim() ?? slug;
}

const link = (slug: string | undefined): PageLink | undefined =>
	slug ? { slug, title: readTitle(slug) } : undefined;

export function getNavigation(): NavigationSection[] {
	return navigation.map((section) => ({
		title: section.title,
		pages: section.pages.map((slug) => ({ slug, title: readTitle(slug) })),
	}));
}

export function getNeighbours(slug: string) {
	const index = ORDER.indexOf(slug);
	return { previous: link(ORDER[index - 1]), next: link(ORDER[index + 1]) };
}

export const PAGE_SLUGS = ORDER;
