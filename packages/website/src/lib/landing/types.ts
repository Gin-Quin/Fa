/** A part of a signature (its return type, its errors...), as colored tokens. */
export type SignatureSegment = {
	kind: string | undefined;
	tokens: Array<{ content: string; color: string; bold: boolean }>;
};
