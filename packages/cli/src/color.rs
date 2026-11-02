//! Colors of the diagnostics printed by the commands: on a terminal, the
//! severity and the carets are red (errors) or yellow (warnings), the
//! gutter blue and the notes cyan. Never when the error output isn't a
//! terminal (a pipe, a file, the tests), nor when `NO_COLOR` is set
//! (<https://no-color.org>). The text is the one of
//! `fa_analyzer::diagnostics::render`, colored line by line.

use std::io::{IsTerminal as _, Write as _};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[1;31m";
const YELLOW: &str = "\x1b[1;33m";
const BLUE: &str = "\x1b[1;34m";
const CYAN: &str = "\x1b[1;36m";

/// Whether the error output gets colors.
fn enabled() -> bool {
	std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
		&& std::io::stderr().is_terminal()
}

/// Prints rendered diagnostics (or any message) to the error output,
/// colored on a terminal.
pub(crate) fn eprint_diagnostics(text: &str) {
	if text.is_empty() {
		return;
	}
	let mut stderr = std::io::stderr().lock();
	let _ = if enabled() {
		stderr.write_all(colorize(text).as_bytes())
	} else {
		stderr.write_all(text.as_bytes())
	};
}

/// [`eprint_diagnostics`] of a message, then a line break.
pub(crate) fn eprintln_message(message: &str) {
	eprint_diagnostics(message);
	eprint_diagnostics("\n");
}

/// The color of a severity (`error`, `warning`), if the text names one.
fn severity_color(text: &str) -> Option<&'static str> {
	if text.starts_with("error") {
		Some(RED)
	} else if text.starts_with("warning") {
		Some(YELLOW)
	} else {
		None
	}
}

/// Colors rendered diagnostics, line by line.
pub(crate) fn colorize(text: &str) -> String {
	let mut out = String::with_capacity(text.len() + text.len() / 4);
	// the color of the last header, for its carets
	let mut accent = RED;
	for line in text.split_inclusive('\n') {
		let body = line.trim_end_matches('\n');
		let newline = &line[body.len()..];
		colorize_line(body, &mut accent, &mut out);
		out.push_str(newline);
	}
	out
}

fn colorize_line(line: &str, accent: &mut &'static str, out: &mut String) {
	// `  12 | source` and `     | ^^^^`
	if let Some(rest) = line.strip_prefix("  ")
		&& let Some(bar) = rest.find(" | ")
		&& rest[..bar].chars().all(|c| c.is_ascii_digit() || c == ' ')
	{
		let gutter = &line[..2 + bar + 3];
		let after = &line[gutter.len()..];
		out.push_str(&format!("{BLUE}{gutter}{RESET}"));
		let carets = after.trim_start_matches([' ', '\t']);
		if rest[..bar].trim().is_empty() && !carets.is_empty() && carets.chars().all(|c| c == '^') {
			out.push_str(&after[..after.len() - carets.len()]);
			out.push_str(&format!("{accent}{carets}{RESET}"));
		} else {
			out.push_str(after);
		}
		return;
	}
	// `path:1:2: error[CODE]: message`, or `error[CODE]: message`
	let label = line
		.find(": error[")
		.or_else(|| line.find(": warning["))
		.map(|index| index + 2)
		.or_else(|| severity_color(line).map(|_| 0))
		.filter(|&start| line[start..].contains("]: "));
	if let Some(start) = label
		&& let Some(color) = severity_color(&line[start..])
		&& let Some(end) = line[start..].find("]: ").map(|end| start + end + 2)
	{
		*accent = color;
		if start > 0 {
			out.push_str(&format!("{BOLD}{}{RESET}", &line[..start]));
		}
		out.push_str(&format!(
			"{color}{}{RESET}{BOLD}{}{RESET}",
			&line[start..end],
			&line[end..]
		));
		return;
	}
	// `  note: …`, `  help: …`
	for word in ["note:", "help:"] {
		if let Some(rest) = line
			.strip_prefix("  ")
			.and_then(|rest| rest.strip_prefix(word))
		{
			out.push_str(&format!("  {CYAN}{word}{RESET}{rest}"));
			return;
		}
	}
	// the summary: `2 errors, 1 warning`
	let summary = line.split(", ").all(|part| {
		part.split_once(' ').is_some_and(|(count, word)| {
			count.chars().all(|c| c.is_ascii_digit())
				&& matches!(word, "error" | "errors" | "warning" | "warnings")
		})
	});
	if summary && !line.is_empty() {
		out.push_str(&format!("{BOLD}{line}{RESET}"));
		return;
	}
	out.push_str(line);
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The colors written as `<name>` tags, to read the tests.
	fn tagged(text: &str) -> String {
		colorize(text)
			.replace(RESET, "</>")
			.replace(BOLD, "<b>")
			.replace(RED, "<red>")
			.replace(YELLOW, "<yellow>")
			.replace(BLUE, "<blue>")
			.replace(CYAN, "<cyan>")
	}

	#[test]
	fn colors_a_rendered_error() {
		let text = "main.fa:2:15: error[NUM010]: 200 doesn't fit\n  2 | let e: Int8 = 100 + 100\n    | \t^^^\n  note: declared here (main.fa:1:5)\n1 error\n";
		assert_eq!(
			tagged(text),
			"<b>main.fa:2:15: </><red>error[NUM010]:</><b> 200 doesn't fit</>\n\
			 <blue>  2 | </>let e: Int8 = 100 + 100\n\
			 <blue>    | </>\t<red>^^^</>\n\
			 \x20 <cyan>note:</> declared here (main.fa:1:5)\n\
			 <b>1 error</>\n"
		);
	}

	#[test]
	fn colors_warnings_in_yellow() {
		let text = "a.fa:1:1: warning[W001]: unused\n  1 | let a = 1\n    | ^\n1 warning\n";
		let tagged = tagged(text);
		assert!(tagged.contains("<yellow>warning[W001]:</>"), "{tagged}");
		assert!(tagged.contains("<yellow>^</>"), "{tagged}");
	}

	#[test]
	fn colors_a_message_with_a_code() {
		assert_eq!(
			tagged("error[CLI001]: no `package.fa`\n  help: create one\n"),
			"<red>error[CLI001]:</><b> no `package.fa`</>\n  <cyan>help:</> create one\n"
		);
	}

	#[test]
	fn leaves_other_lines_unchanged() {
		let text = "fa check: package `api` (packages/api)\n  (for the target `web`)\n\n";
		assert_eq!(colorize(text), text);
	}
}
