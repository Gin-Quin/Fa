export type TocEntry = { id: string; text: string; depth: 2 | 3 };

/** A part of a page, between two headings: used by the search. */
export type PageSection = { id: string; heading: string; text: string };

export type PageLink = { slug: string; title: string };

export type NavigationSection = { title: string; pages: PageLink[] };
