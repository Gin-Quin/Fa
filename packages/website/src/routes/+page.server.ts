import { highlight, tokenize } from "#lib/highlight/highlighter.server.ts";
import {
	agentLoop,
	features,
	heroFiles,
	memory,
	signature,
	signatureParts,
} from "#lib/landing/snippets.ts";
import type { SignatureSegment } from "#lib/landing/types.ts";

import type { PageServerLoad } from "./$types";

/** Splits the first line of the signature into its parts (return type, errors, requirements...). */
async function splitSignature(): Promise<SignatureSegment[]> {
	const [line] = signature.split("\n");
	const ranges = signatureParts.map((part) => {
		const start = line.indexOf(part.source);
		if (start < 0) {
			throw new Error(`"${part.source}" is not part of the signature`);
		}
		return { kind: part.kind as string, start, end: start + part.source.length };
	});

	const segments: SignatureSegment[] = [];
	for (const token of await tokenize(line)) {
		// tokens may span several parts: they are cut at the boundaries of the parts
		let offset = token.offset;
		let content = token.content;
		while (content.length > 0) {
			const range = ranges.find((candidate) => offset >= candidate.start && offset < candidate.end);
			const next = ranges.map((candidate) => candidate.start).find((start) => start > offset);
			const limit = range ? range.end : (next ?? Number.POSITIVE_INFINITY);
			const piece = content.slice(0, limit - offset);
			const kind = range?.kind;
			const last = segments.at(-1);
			const styled = { content: piece, color: token.color, bold: token.bold };
			if (last && last.kind === kind) {
				last.tokens.push(styled);
			} else {
				segments.push({ kind, tokens: [styled] });
			}
			offset += piece.length;
			content = content.slice(piece.length);
		}
	}
	return segments;
}

const highlightAll = async <Key extends string>(snippets: Record<Key, string>) =>
	Object.fromEntries(
		await Promise.all(
			Object.entries<string>(snippets).map(async ([key, code]) => [key, await highlight(code)]),
		),
	) as Record<Key, string>;

export const load: PageServerLoad = async () => ({
	hero: await Promise.all(
		heroFiles.map(async (file) => ({
			name: file.name,
			label: file.label,
			diagnostic: file.diagnostic,
			html: await highlight(file.code),
		})),
	),
	signature: {
		segments: await splitSignature(),
		body: await highlight(signature.split("\n").slice(1).join("\n")),
		parts: await Promise.all(
			signatureParts.map(async (part) => ({
				kind: part.kind,
				label: part.label,
				title: part.title,
				text: part.text,
				call: await highlight(part.call),
			})),
		),
	},
	features: await highlightAll(features),
	memory: {
		invalidation: await highlight(memory.invalidation, { errorLine: memory.invalidationErrorLine }),
		precise: await highlight(memory.precise),
	},
	agentLoop: await highlightAll(agentLoop),
});
