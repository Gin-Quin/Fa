#pragma once

namespace Help {
	void infos() {
		cout << Font::bold << "🎺  Fa compiler" << Font::reset << " "
			  << Font::bold << Ink::brightGreen << version.MAJOR
			  << Font::reset << "."
			  << Font::bold << Ink::brightCyan << version.MINOR
			  << Font::reset << "."
			  << Font::bold << Ink::brightYellow << version.PATCH
			  << Font::reset << "  🎺"
			  << endl;
	}

	void command(const char* command) {
		cout << Font::bold << Ink::brightCyan << command << Font::reset << ' ';
	}

	void action(const char* action) {
		cout << Font::bold <<action << Font::reset << ' ';
	}

	void parameter(const char* parameter) {
		cout << Font::bold << "{ " << parameter << " }" << Font::reset << ' ';
	}

	void optionalParameter(const char* parameter) {
		cout << Ink::white << "{ " << parameter << " }" << Font::reset << ' ';
	}


	void option(const char* option) {
		cout << Ink::white << "[--" << option << "]" << Font::reset << ' ';
	}

	void comment(const char* comment) {
		cout << endl << Ink::white << "   ─ " << comment << Font::reset << ' ' << endl;
	}

	void usage() {
		cout << endl << Font::bold << "────────  Usage  ────────" << Font::reset << endl;

		command("fa");
			parameter("filename");
		comment("Execute a .fa file");
		cout << endl;

		command("fa");
			action("compile");
			parameter("inputFile");
			parameter("...outputFiles");
			option("bundle");
		comment("Compile one or more output files from a single input file");
		cout << endl;

		command("fa");
			action("parse");
			parameter("inputFile");
			optionalParameter("outputFile");
			option("tokens");
			option("raw-ast");
			option("ast");
			option("dependencies");
			option("declarations");
			option("js");
			option("cpp");
			option("wasm");
		comment("Parse a single input file with a list of options and output JSON in the given output file - or in stdout if no output file");
		cout << Font::bold << "─────────────────────────" << Font::reset << endl << endl;
	}
}
