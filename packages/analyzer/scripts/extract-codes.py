#!/usr/bin/env python3
"""Generates the diagnostic code registry from the three implementation briefs.

Usage (from the repository root):

    python3 packages/analyzer/scripts/extract-codes.py

Every code-shaped token of the briefs (`E-NUM-10`, `TY002`, `REF001`…) is
collected, ranges such as `TY010`–`TY012` are expanded, and each code is
written to the registry file of the brief that owns its prefix (decision A6 of
`design/analyzer-design.md`):

- values brief   → `src/diagnostics/codes/values.rs`   (`E-*`, `W-*`)
- types brief    → `src/diagnostics/codes/types.rs`    (`TY GN TF CV FN EF CL`)
- advanced brief → `src/diagnostics/codes/advanced.rs` (`EFF REF MOD REQ …`)

Codes listed in `ALIASES` are exact duplicates: they are not defined on their
own, they become aliases of their canonical code. The summary of a code is the
condition of its table row or index entry, or else the sentence that mentions
it first. The severity is `Warning` for `W`-prefixed codes and for codes the
advanced brief calls warnings, `Error` otherwise.

The generated files are committed; re-run the script when a brief changes.
"""

import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[3]
BRIEFS = ROOT / "design" / "implementation"
OUT = ROOT / "packages" / "analyzer" / "src" / "diagnostics" / "codes"

SOURCES = {
	"values": BRIEFS / "values-and-control-flow.md",
	"types": BRIEFS / "types-functions-collections.md",
	"advanced": BRIEFS / "advanced-and-architecture.md",
}

PATTERNS = {
	"values": r"[EW]-[A-Z]+-\d+",
	"types": r"W?(?:TY|GN|TF|CV|FN|EF|CL)\d{3}",
	"advanced": r"W?(?:EFF|REF|REL|REQ|STR|REA|META|FFI|CONC|MOD|CFG|DEP|TEST|CLI|MEM)\d{3}",
}

ANY_CODE = "(?:" + "|".join(PATTERNS.values()) + ")"

# alias → canonical. Decision A6 plus the exact duplicates found when reading
# the definitions side by side (see `tests/code-map.txt` for the overlaps that
# are not exact and stay distinct codes).
ALIASES = {
	# listed in decision A6
	"EF015": "E-ERR-3",
	"EF030": "REF013",
	"E-DECL-16": "REF001",
	"FN010": "EFF020",
	"CV001": "E-NUM-11",
	"CL060": "E-NUM-17",
	"WEF002": "W-ERR-1",
	"E-TYPE-2": "TY040",
	# the reference-checker codes the types brief borrows from the advanced brief
	"EF031": "REF012",
	"EF032": "REF001",
	# `use`/`mutable use`/`reuse` rules of the values brief, owned by the advanced brief
	"E-DECL-7": "REF004",
	"E-DECL-17": "REF006",
	"E-DECL-18": "REF005",
	"E-DECL-19": "REF011",
	"FN071": "REF011",
	# parameter mutability, owned by the advanced brief (effects)
	"E-DECL-5": "EFF007",
	"EF011": "EFF007",
	"E-DECL-13": "EFF006",
	"EF013": "EFF006",
	"E-DECL-14": "EFF005",
	"EF014": "EFF005",
	# errors, owned by the values brief
	"EF017": "E-ERR-6",
	"EF021": "E-ERR-1",
	# decided by the lead (design/decisions.md, "Analyzer"): E-TYPE-1 is the
	# canonical type mismatch, TY002 names the same condition
	"TY002": "E-TYPE-1",
	# `assert` with a non-Boolean condition: a case of E-BOOL-1 (values brief §8)
	"TEST001": "E-BOOL-1",
	# exact duplicates settled by the lead (design/decisions.md, "Analyzer")
	"TY009": "E-DECL-2",
	"FN001": "E-DECL-6",
	"FN014": "E-DO-2",
	"TY070": "E-DECL-9",
	# objects: the types brief §3.2.5 and the values brief §11 name the same conditions
	"TY005": "E-OBJ-6",
	"TY011": "E-OBJ-12",
	"TY014": "E-OBJ-1",
	"TY015": "E-OBJ-2",
	"TY016": "E-OBJ-4",
	"TY017": "E-OBJ-3",
	"TY018": "E-OBJ-7",
	"TY019": "E-OBJ-5",
	"TY020": "E-OPT-1",
	"TY022": "E-OBJ-10",
	"TY023": "E-DECL-4",
	"TY024": "E-OBJ-11",
	"WTY001": "W-OBJ-1",
	# an unqualified member in a pattern: a case of TY057 (members are always qualified)
	"E-CMP-9": "TY057",
	# enumerations are not ordered: a case of E-CMP-2
	"TY085": "E-CMP-2",
	# an empty list without context: a case of E-DECL-11
	"CL001": "E-DECL-11",
	# Set elements and Map keys are exact: both briefs name the condition
	"STR006": "CL010",
	# structural typing (advanced brief §10) names the object rules of both briefs
	"STR002": "E-OBJ-1",
	"STR008": "E-OBJ-12",
	"TY026": "STR003",
	# a nominal type given for another: the types brief calls it a mismatch (TY002)
	"STR009": "E-TYPE-1",
	# decided by the lead (design/decisions.md, "Analyzer", Codes)
	"TY006": "E-IF-2",
	"TY025": "E-CMP-4",
}

WARNING_HINTS = [
	r"warning\s+`?({code})",
	r"({code})`?\s*\(warning",
	r"--\s*warning:?\s+({code})",
]


def owner_of(code):
	for brief, pattern in PATTERNS.items():
		if re.fullmatch(pattern, code):
			return brief
	raise ValueError(code)


def clean(text):
	text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
	text = text.replace("**", "").replace("`", "").replace("\\|", "|")
	text = re.sub(r"\s+", " ", text).strip(" .;,:-—")
	if len(text) > 140:
		text = text[:140].rsplit(" ", 1)[0] + "…"
	return text


def expand_ranges(text):
	"""Codes written as ranges: `TY010`–`TY012`."""
	codes = set()
	for match in re.finditer(r"`?(" + ANY_CODE + r")`?\s*[–-]\s*`?(" + ANY_CODE + r")`?", text):
		first, last = match.group(1), match.group(2)
		prefix_a, number_a = re.fullmatch(r"(.*?)(\d+)", first).groups()
		prefix_b, number_b = re.fullmatch(r"(.*?)(\d+)", last).groups()
		if prefix_a != prefix_b or int(number_b) <= int(number_a) or int(number_b) - int(number_a) > 20:
			continue
		width = len(number_a)
		for number in range(int(number_a), int(number_b) + 1):
			codes.add(f"{prefix_a}{number:0{width}d}" if not prefix_a.endswith("-") else f"{prefix_a}{number}")
	return codes


def summaries(text):
	"""The best summary of each code mentioned in `text`.

	Ranks, best first: 0 a table row whose first cell is the code; 1 an entry
	"`CODE` condition" of an index line; 2 a clause ending with the code
	("condition `CODE`"); 3 an entry "`CODE` condition" or a quoted message
	elsewhere; 4 a generic index entry ("… (§3.5)"); 5 the line that mentions it.
	"""
	found = {}

	def add(code, summary, rank):
		summary = clean(summary)
		if re.search(r"\(§[\d.]+\)$", summary):
			rank = max(rank, 4)
		if len(summary) >= 6 and (code not in found or found[code][0] > rank):
			found[code] = (rank, summary)

	code_group = r"`(" + ANY_CODE + r")`((?:\s*[–/,]\s*`" + ANY_CODE + r"`)*)"
	entry = code_group + r"\s+((?:(?!·|\||`" + ANY_CODE + r"`).)+)"
	for line in text.splitlines():
		row = re.match(r"^\|([^|]+)\|([^|]+)\|", line)
		if row:
			cell = row.group(1)
			codes = re.findall(ANY_CODE, cell)
			if codes and re.fullmatch(r"[\s`/,–-]*(?:" + ANY_CODE + r"[\s`/,–-]*)+", cell):
				for code in set(codes) | expand_ranges(cell):
					add(code, row.group(2), 0)
			continue
		is_index = line.startswith("- **")
		for match in re.finditer(entry, line):
			codes = {match.group(1)} | set(re.findall(ANY_CODE, match.group(2))) | expand_ranges(match.group(0))
			summary = match.group(3)
			quoted = re.match(r'^"([^"]+)"', summary)
			if quoted:
				summary = quoted.group(1)
			elif summary.startswith(("(", ",", ")", "and ", "or ", ":", "=", "→", "⇒")):
				continue
			for code in codes:
				add(code, summary, 1 if is_index else 3)
		for match in re.finditer(r"(?:^|[;:.(]\s|, )([^;:.()`\"]*(?:`[^`]*`[^;:.()`\"]*)*?)\s*\(?`(" + ANY_CODE + r")`\)?\s*(?=[;,.)]|$)", line):
			clause = match.group(1)
			if len(clause.split()) >= 2 and not re.search(ANY_CODE, clause):
				add(match.group(2), clause, 2)
		for match in re.finditer(r"(" + ANY_CODE + r")`?\)?\s*\"([^\"]+)\"", line):
			add(match.group(1), match.group(2), 3)
		for code in re.findall(ANY_CODE, line):
			add(code, re.sub(r"^\s*[-|>]\s*", "", line), 5)
	return {code: summary for code, (rank, summary) in found.items()}


def main():
	texts = {brief: path.read_text() for brief, path in SOURCES.items()}
	everything = "\n".join(texts.values())

	codes = set()
	for pattern in PATTERNS.values():
		codes |= set(re.findall(r"(?<![\w-])(" + pattern + r")(?![\w-])", everything))
	codes |= expand_ranges(everything)

	per_brief = {brief: summaries(text) for brief, text in texts.items()}

	warnings = set()
	for code in codes:
		if code.startswith("W"):
			warnings.add(code)
	for hint in WARNING_HINTS:
		for match in re.finditer(hint.format(code=ANY_CODE), texts["advanced"], re.IGNORECASE):
			warnings.add(match.group(1))

	missing = [alias for alias in ALIASES if alias not in codes] + [
		canonical for canonical in ALIASES.values() if canonical not in codes
	]
	if missing:
		raise SystemExit(f"aliases name unknown codes: {missing}")

	aliases_of = {}
	for alias, canonical in ALIASES.items():
		aliases_of.setdefault(canonical, []).append(alias)

	registry = {brief: [] for brief in SOURCES}
	for code in codes:
		if code in ALIASES:
			continue
		brief = owner_of(code)
		summary = per_brief[brief].get(code)
		if summary is None:
			summary = next(
				(per_brief[other][code] for other in SOURCES if code in per_brief[other]),
				f"see the {brief} brief",
			)
		registry[brief].append((code, summary))

	def sort_key(entry):
		prefix, number = re.fullmatch(r"(.*?)(\d+)", entry[0]).groups()
		return (prefix, int(number))

	for brief, entries in registry.items():
		entries.sort(key=sort_key)
		out = [
			f"//! Diagnostic codes owned by the {brief} brief (`{SOURCES[brief].relative_to(ROOT)}`).",
			"//!",
			"//! Generated by `packages/analyzer/scripts/extract-codes.py`: edit the",
			"//! script or the brief, not this file.",
			"",
			"use crate::diagnostics::Severity::{Error, Warning};",
			"",
			"codes! {",
		]
		for code, summary in entries:
			name = code.replace("-", "_")
			severity = "Warning" if code in warnings else "Error"
			summary = summary.replace("\\", "\\\\").replace('"', '\\"')
			aliases = ", ".join(f'"{alias}"' for alias in sorted(aliases_of.get(code, [])))
			out.append(f'\t{name} = "{code}", {severity}, [{aliases}], "{summary}";')
		out.append("}")
		(OUT / f"{brief}.rs").write_text("\n".join(out) + "\n")
		print(f"{brief}: {len(entries)} codes")
	print(f"aliases: {len(ALIASES)}")


if __name__ == "__main__":
	main()
