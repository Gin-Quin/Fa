#pragma once
/**
* When given a string with an offset pointing to a group character like '[', '(', '{', '\' or '"',
* return the length until the group close.
*/


int fragment(const char* src, int offset=0) {
	int cursor = offset;
	char c;
	char starter = src[offset], ender;

	switch (starter) {
		case 0: return 0;  // empty string

		case '"': case '\'': case '`': case '/':  // string / regular expression
			while ((c = src[++cursor])) switch (c) {
				case starter: return cursor - offset;
				case '\\': if (src[cursor+1]) cursor++;  // next character is escaped
			}
			throw ((string) "Missing ") + starter;

		case '(': ender = ')'; break;
		case '[': ender = ']'; break;
		case '{': ender = '}'; break;
	}

	// groups who can contain sub-groups and strings
	int level = 0;
	while ((c = src[++cursor])) switch (c) {
		case starter: level++; continue;
		case ender:
			if (level) level--;
			else return cursor - offset;
		continue;

		case '"': case '\'': case '`': case '/':  // we enter a string or regex
			char subStarter = c;
			while ((c = src[++cursor]) && c != subStarter)
				if (c == '\\' && src[cursor+1]) cursor++;  // next character is escaped
			if (!c) throw ((string) "Missing ") + subStarter;
	}

	throw ((string) "Missing ") + ender;
}
