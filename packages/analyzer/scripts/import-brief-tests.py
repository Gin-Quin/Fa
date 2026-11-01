#!/usr/bin/env python3
"""Imports the numbered test cases of the three implementation briefs into
`packages/analyzer/tests/cases/<brief>/<section>/<id>.fa`.

Usage (from the repository root):

    python3 packages/analyzer/scripts/import-brief-tests.py

- values brief: the `- T-AREA-n` lines (` · ` separates lines, `⇥` is a tab,
  `→` gives the output, `⇒` the diagnostics). A line with several programs
  (`valid; … ⇒ E-LEX-4`) gives one case per program, suffixed `-a`, `-b`…
- types brief: the `-- ✓ id` / `-- ✗ id` cases of its ```fa blocks. A case may
  reuse the declarations of the earlier cases of its block: the ones it names
  are copied in front of it.
- advanced brief: every ```fa block of a `Tests` section, with its own inline
  `-- error CODE` annotations. The declarations of `Pet`, `Person` and
  `Player` are added when a case names them without declaring them.

Every case gets a `-- pending: M1|M2|M3 (imported from the … brief)` header
chosen from the acceptance lists of `design/analyzer-design.md` §5. Expected
diagnostics whose line the brief doesn't give are written as standalone
annotation lines (`-- error: TY002`), which match anywhere in the file; a
reviewer pins them to their line when the case stops being pending.

Codes are mapped through `tests/code-map.txt` (`FROM -> TO` lines). The
script rewrites the whole `tests/cases/{values,types,advanced}` directories.
"""

import pathlib
import re
import shutil

ROOT = pathlib.Path(__file__).resolve().parents[3]
BRIEFS = ROOT / "design" / "implementation"
ANALYZER = ROOT / "packages" / "analyzer"
CASES = ANALYZER / "tests" / "cases"
CODE_MAP = ANALYZER / "tests" / "code-map.txt"

CODE = r"(?:[EW]-[A-Z]+-\d+|W?(?:TY|GN|TF|CV|FN|EF|CL|EFF|REF|REL|REQ|STR|REA|META|FFI|CONC|MOD|CFG|DEP|TEST|CLI|MEM|AN)\d{3})"

ADVANCED_PRELUDE = {
	"Pet": "type Pet = { name: String }",
	"Person": "type Person = { name: String, age: Integer = 0, pets: Array(Pet) = [] }",
	"Player": "type Player = { name: String, score: Integer }",
}

DECLARATION = re.compile(r"^(?:export\s+)?(?:type|let|mutable|static|function|namespace)\s+([A-Za-z_#]\w*)")


def slug(text):
	return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")


def load_code_map():
	mapping = {}
	if CODE_MAP.exists():
		for line in CODE_MAP.read_text().splitlines():
			line = line.split("#", 1)[0].strip()
			if "->" in line:
				source, target = (part.strip() for part in line.split("->", 1))
				mapping[source] = target
	return mapping


def load_severities():
	severities = {}
	for path in (ANALYZER / "src" / "diagnostics" / "codes").glob("*.rs"):
		for match in re.finditer(r'= "([^"]+)", (Error|Warning), \[([^\]]*)\]', path.read_text()):
			severity = match.group(2).lower()
			severities[match.group(1)] = severity
			for alias in re.findall(r'"([^"]+)"', match.group(3)):
				severities[alias] = severity
	return severities


CODE_MAPPING = load_code_map()
SEVERITIES = load_severities()


def map_code(code):
	return CODE_MAPPING.get(code, code)


def severity(code):
	return SEVERITIES.get(code) or ("warning" if code.startswith("W") else "error")


def without_parentheses(text):
	previous = None
	while previous != text:
		previous = text
		text = re.sub(r"\([^()]*\)", "", text)
	return text


def write_case(brief, section, name, header, body):
	directory = CASES / brief / section
	directory.mkdir(parents=True, exist_ok=True)
	content = "\n".join(header) + "\n" + body.rstrip("\n") + "\n"
	(directory / f"{name}.fa").write_text(content)


# ── values brief ──────────────────────────────────────────────────────────


def in_ranges(number, ranges):
	return any(low <= number <= high for low, high in ranges)


def values_milestone(test_id, program, codes):
	area, number = re.fullmatch(r"T-([A-Z]+)-(\d+)", test_id).groups()
	number = int(number)
	m1_numbers = [(1, 14), (17, 18), (21, 25), (41, 43), (45, 46), (49, 53), (60, 80), (82, 83), (86, 86), (89, 90),
		(19, 20), (33, 36), (54, 54)]
	if area == "DECL":
		if in_ranges(number, [(1, 5), (20, 30)]):
			return "M1"
		if in_ranges(number, [(6, 13), (35, 37)]):
			return "M2"
		return "M2" if re.search(r"\b(use|reuse|mutates|invalidates)\b", program) else "M1"
	if area == "NUM":
		return "M1" if in_ranges(number, m1_numbers) else "M2"
	if area == "STR":
		return "M1" if number <= 13 else "M2"
	if area in ("TUP", "RNG", "DST"):
		return "M2"
	if area == "OBJ":
		return "M2" if ("..." in program or ".=" in program) else "M1"
	if area == "NAR":
		return "M2" if re.search(r"\b(use|reuse|mutates|invalidates)\b", program) else "M1"
	if area == "LOOP":
		return "M2" if "E-LOOP-4" in codes else "M1"
	if area in ("IF", "BOOL", "CMP", "OPT", "WHEN", "ERR", "DO", "LEX"):
		return "M1"
	return "M2"


def split_top_level(text, separator="; "):
	"""Splits on `separator` outside backticks."""
	parts, current, in_code, i = [], [], False, 0
	while i < len(text):
		if text[i] == "`":
			in_code = not in_code
		if not in_code and text.startswith(separator, i):
			parts.append("".join(current))
			current = []
			i += len(separator)
			continue
		current.append(text[i])
		i += 1
	parts.append("".join(current))
	return parts


def program_lines(snippet):
	return snippet.replace(" · ", "\n").replace("·", "\n").replace("⇥", "\t")


def import_values():
	text = (BRIEFS / "values-and-control-flow.md").read_text()
	section = "00-misc"
	count = 0
	for line in text.splitlines():
		heading = re.match(r"^## (\d+)\. (.*)", line)
		if heading:
			section = f"{int(heading.group(1)):02d}-{slug(heading.group(2))}"
			continue
		match = re.match(r"^- (T-[A-Z]+-\d+) (.*)$", line)
		if not match:
			continue
		test_id, rest = match.groups()
		variants = []
		for segment in split_top_level(rest):
			snippet = re.match(r"\s*`([^`]*)`(.*)", segment)
			if snippet:
				variants.append({"code": snippet.group(1), "tail": snippet.group(2), "review": None})
			elif variants:
				variants[-1]["tail"] += "; " + segment
			else:
				anywhere = re.search(r"`([^`]*)`(.*)", segment)
				variants.append({
					"code": anywhere.group(1) if anywhere else "",
					"tail": anywhere.group(2) if anywhere else segment,
					"review": f"not self-contained in the brief: {segment.strip()}",
				})
		for index, variant in enumerate(variants):
			name = test_id if len(variants) == 1 else f"{test_id}-{'abcdefghij'[index]}"
			tail = variant["tail"]
			program = program_lines(variant["code"])
			codes = [map_code(code) for code in re.findall(CODE, without_parentheses(tail))]
			milestone = values_milestone(test_id, program, codes)
			header = [f"-- pending: {milestone} (imported from the values brief)"]
			if variant["review"]:
				header.append(f"-- review: {variant['review']}")
			for code in codes:
				header.append(f"-- {severity(code)}: {code}")
			footer = []
			output = re.search(r"→\s*`([^`]*)`", tail)
			if output:
				footer += [f"-- prints: {printed}" for printed in output.group(1).split(" / ")]
			if re.search(r"⇒\s*stop", tail):
				footer.append("-- exit: 101")
			elif re.search(r"⇒\s*exit 1", tail):
				footer.append("-- exit: 1")
			note = re.sub(r"\s+", " ", tail).strip()
			if note:
				header.append(f"-- brief: {note}")
			write_case("values", section, name, header, program + ("\n" + "\n".join(footer) if footer else ""))
			count += 1
	return count


# ── types brief ───────────────────────────────────────────────────────────

ORDINALS = {"first": 1, "second": 2, "third": 3, "fourth": 4, "fifth": 5}


def types_milestone(section, body):
	major = tuple(int(part) for part in section.split("-", 1)[0].split("."))
	if major[:2] in [(3, 12), (3, 14), (5, 5), (5, 6)]:
		return "M3"
	if major[:2] in [(3, 6), (3, 9), (3, 10), (3, 11)] or major[0] == 5 or (major[0] == 4 and major[1] >= 3):
		return "M2"
	if re.search(r"\btype \w+\([A-Z]|\bType\b", body):
		return "M2"
	return "M1"


def top_level_statements(lines):
	"""Groups lines into statements by bracket depth."""
	statements, current, depth = [], [], 0
	for line in lines:
		if depth == 0 and current and not line.startswith(("\t", " ", ".", "}", ")", "]")):
			statements.append(current)
			current = []
		current.append(line)
		code = re.sub(r'"(?:\\.|[^"\\])*"', '""', line.split("--", 1)[0])
		depth += code.count("{") + code.count("(") + code.count("[") - code.count("}") - code.count(")") - code.count("]")
		depth = max(depth, 0)
	if current:
		statements.append(current)
	return statements


def declared_names(statements):
	names = {}
	for statement in statements:
		match = DECLARATION.match(statement[0])
		if match:
			names[match.group(1)] = statement
	return names


def code_lines(lines):
	return [i for i, line in enumerate(lines) if line.strip() and not line.strip().startswith("--")]


def annotate(lines, index, annotation):
	if "--" in lines[index].split('"')[-1]:
		lines[index] += f", {annotation}"
	else:
		lines[index] += f" -- {annotation}"


def import_types():
	text = (BRIEFS / "types-functions-collections.md").read_text()
	section = "0-misc"
	count = 0
	lines = text.splitlines()
	i = 0
	while i < len(lines):
		line = lines[i]
		heading = re.match(r"^### (\d+\.\d+) (.*)", line)
		if heading:
			section = f"{heading.group(1)}-{slug(heading.group(2))}"
		if line.strip() != "```fa":
			i += 1
			continue
		block = []
		i += 1
		while i < len(lines) and lines[i].strip() != "```":
			block.append(lines[i])
			i += 1
		i += 1
		cases = []
		for block_line in block:
			case = re.match(r"^-- ([✓✗]) ([\w.-]+):?\s*(.*)$", block_line)
			if case:
				cases.append({"valid": case.group(1) == "✓", "id": case.group(2), "description": case.group(3), "lines": []})
			elif cases:
				cases[-1]["lines"].append(block_line)
		earlier = []
		for case in cases:
			body_lines = list(case["lines"])
			while body_lines and not body_lines[-1].strip():
				body_lines.pop()
			description = case["description"]
			summary, _, output = description.partition("→")
			codes = [map_code(code) for code in re.findall(CODE, summary)]
			if case["valid"]:
				codes = [code for code in codes if severity(code) == "warning"]
			# line placement
			header_codes = list(codes)
			positions = code_lines(body_lines)
			ordinal = re.search(r"on the (first|second|third|fourth|fifth|last) line", summary)
			if positions and codes and ordinal:
				word = ordinal.group(1)
				target = positions[-1] if word == "last" else positions[min(ORDINALS[word], len(positions)) - 1]
				for code in codes:
					annotate(body_lines, target, f"{severity(code)}: {code}")
				header_codes = []
			elif positions and len(codes) == 1 and "(both lines)" in summary and len(positions) >= 2:
				for target in positions[-2:]:
					annotate(body_lines, target, f"{severity(codes[0])}: {codes[0]}")
				header_codes = []
			elif positions and len(codes) > 1 and "one per line" in summary and len(positions) >= len(codes):
				for code, target in zip(codes, positions[-len(codes):]):
					annotate(body_lines, target, f"{severity(code)}: {code}")
				header_codes = []
			# declarations reused from earlier cases of the block
			statements = top_level_statements(body_lines)
			own = declared_names(statements)
			available = {}
			for previous in earlier:
				available.update(declared_names(top_level_statements(previous["lines"])))
			used = set(re.findall(r"[A-Za-z_]\w*", "\n".join(line.split("--", 1)[0] for line in body_lines)))
			needed, frontier = [], [name for name in used if name in available and name not in own]
			while frontier:
				name = frontier.pop()
				if name in needed:
					continue
				needed.append(name)
				statement_text = "\n".join(available[name])
				for other in re.findall(r"[A-Za-z_]\w*", statement_text):
					if other in available and other not in own and other not in needed:
						frontier.append(other)
			context = []
			if needed:
				sources = sorted({c["id"] for c in earlier if set(declared_names(top_level_statements(c["lines"]))) & set(needed)})
				context.append(f"-- context: declarations reused from {', '.join(sources)}")
				order = []
				for previous in earlier:
					for statement in top_level_statements(previous["lines"]):
						match = DECLARATION.match(statement[0])
						if match and match.group(1) in needed:
							order = [s for s in order if DECLARATION.match(s[0]).group(1) != match.group(1)]
							order.append([re.sub(r"\s+--\s.*$", "", l) for l in statement])
				for statement in order:
					context.extend(statement)
			milestone = types_milestone(section, "\n".join(body_lines))
			header = [f"-- pending: {milestone} (imported from the types brief)"]
			header += [f"-- {severity(code)}: {code}" for code in header_codes]
			header.append(f"-- brief: {'✓' if case['valid'] else '✗'} {case['id']}: {description}")
			footer = []
			if output.strip():
				footer = [f"-- prints: {printed.strip()}" for printed in output.split(" / ")]
			body = "\n".join(context + body_lines + footer)
			write_case("types", section, case["id"], header, body)
			earlier.append(case)
			count += 1
	return count


# ── advanced brief ────────────────────────────────────────────────────────


def advanced_milestone(number, body):
	if number == "18.4" and re.search(r"TEST00[12]", body):
		return "M1"
	if number in ("3.8", "4.5", "5.13", "6.8", "9.9", "10.8", "15.9", "18.4"):
		return "M2"
	return "M3"


def import_advanced():
	text = (BRIEFS / "advanced-and-architecture.md").read_text()
	lines = text.splitlines()
	chapter, tests, count, index = "00-misc", None, 0, 0
	i = 0
	while i < len(lines):
		line = lines[i]
		heading = re.match(r"^## (\d+)\. (.*)", line)
		if heading:
			chapter = f"{int(heading.group(1)):02d}-{slug(heading.group(2))}"
			tests = None
		subheading = re.match(r"^### (\d+\.\d+) (.*)", line)
		if subheading:
			tests = subheading.group(1) if "Tests" in subheading.group(2) else None
			index = 0
		if line.strip() != "```fa" or tests is None:
			i += 1
			continue
		block = []
		i += 1
		while i < len(lines) and lines[i].strip() != "```":
			block.append(lines[i])
			i += 1
		i += 1
		index += 1
		body = "\n".join(block)
		for code in set(re.findall(CODE, body)):
			if map_code(code) != code:
				body = re.sub(rf"\b{re.escape(code)}\b", map_code(code), body)
		used = set(re.findall(r"\b[A-Z]\w*\b", body))
		declared = {match.group(1) for match in re.finditer(r"\btype\s+(\w+)", body)}
		prelude = []
		if "-- file:" not in body:
			for name in ("Pet", "Person", "Player"):
				if name in used and name not in declared:
					prelude.append(ADVANCED_PRELUDE[name])
			if ADVANCED_PRELUDE["Person"] in prelude and ADVANCED_PRELUDE["Pet"] not in prelude and "Pet" not in declared:
				prelude.insert(0, ADVANCED_PRELUDE["Pet"])
		milestone = advanced_milestone(tests, body)
		header = [f"-- pending: {milestone} (imported from the advanced brief)"]
		if re.search(r"\btest\s+\"", body):
			header.append("-- kind: test")
		if prelude:
			header.append("-- context: the brief's shared declarations (§1)")
		name = f"adv-{tests}-{index:02d}"
		write_case("advanced", chapter, name, header, "\n".join(prelude + [body]))
		count += 1
	return count


def main():
	for brief in ("values", "types", "advanced"):
		shutil.rmtree(CASES / brief, ignore_errors=True)
	print(f"values: {import_values()} cases")
	print(f"types: {import_types()} cases")
	print(f"advanced: {import_advanced()} cases")


if __name__ == "__main__":
	main()
