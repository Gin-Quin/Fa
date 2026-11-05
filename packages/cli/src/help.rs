//! `fa help [<command>]`: the commands of `fa` (architecture/overview.md,
//! "Command line").

/// The summary of every command.
pub(crate) const HELP: &str = "\
fa: the Fa toolchain

Usage: fa <command> [arguments...]

Commands:
  run      compile a file, a script or a package's application for Bun, and run it
  check    analyze a file, a package or every package of a workspace
  build    compile a file or the application of a package for its targets (into build/<target>/)
  test     run the tests of the `*.test.fa` files (`--watch` to run them again after each change)
  develop  run the application, and reload or restart it after each change
  preview  run the output of `fa build`
  format   format the source files
  update   resolve the dependencies again and rewrite `dependencies.lock`
  new      create a package or a workspace
  bind     write the foreign file of a C header (`fa bind header.h --output file.foreign.fa`)
  help     describe a command: `fa help <command>`
  version  print the version of fa

In a workspace, `check`, `build`, `test`, `develop` and `preview` take the name
of a package; without it, they run for every package that implements them.
The commands exit with 0 when they succeed, and 1 when they fail (`run` and
`preview` exit with the program's code). The native and wasm targets are
compiled by `fa-native`, found next to `fa` or on the PATH.
";

/// The description of one command.
pub(crate) fn command(name: &str) -> Option<&'static str> {
	Some(match name {
		"run" => {
			"\
Usage: fa run [--target=<target>] [--overflow-checks=true|false]
              [--asynchronous=true|false] <file.fa | package> [--] [arguments...]

Compiles a standalone file, a script (`scripts/seed.fa`) or the application of a
package for Bun, and runs it with Bun (found on the PATH or in ~/.bun/bin). The
arguments after the file go to the program; a `--` right after the file is
dropped. `--target=native|wasm` compiles with the LLVM backend instead. The exit
code is the program's. `--asynchronous` overrides `build.asynchronous` (by
default, a program is asynchronous when it starts a task or may wait).
"
		}
		"check" => {
			"\
Usage: fa check [<file.fa> | <package>] [--format=text|json] [--timings]

Analyzes every file (library, application, scripts, tests) and prints the
diagnostics; exits with 1 when there is an error. In a workspace, without a
package, every package is checked. `--format=json` prints them to stdout
instead, one JSON object per line: `file`, `range` (`start` and `end`, with a
1-based `line` and `column`), `severity`, `code`, `message`, and `target` when
only the analysis for that target found it. `--timings` prints the time of each
phase and the sizes of the analysis to stderr.
"
		}
		"build" => {
			"\
Usage: fa build [<file.fa> | <package>] [--target=<target>] [--output=<directory>]
                [--declarations] [--minify=true|false] [--overflow-checks=true|false]
                [-O0|-O1|-O2|-O3] [--size] [--emit-ir] [--timings]
                [--asynchronous=true|false] [-v]

Compiles a standalone file, or the application of a package, into
<output>/<target>/ (`--output`, else `build.output` of the package, `build` by
default). The targets are bun, node, web, native, native-freestanding and wasm:
without `--target`, a package is built for each target of its configuration
(`targets`, by default web, bun and native; a script too), each one's outcome
reported; a standalone file is built for the default targets (bun and native
when it uses a foreign file). The
native and wasm targets are compiled by `fa-native` (next to `fa`, or on the
PATH). In a workspace, without a package, every package with an application is
built.
`--timings` prints the time of each phase and the sizes of the analysis to
stderr. `--asynchronous=true|false` overrides `build.asynchronous` (by default,
a program is asynchronous when its entry starts a task or may wait); `-v` says
which mode the program builds in, and why.
"
		}
		"test" => {
			"\
Usage: fa test [<package>] [--watch] [--target=bun|native] [--asynchronous=true|false]

Runs the tests of the `*.test.fa` files on Bun, each with freshly initialized
module-level values, each test file in a Bun process of its own (as many at once
as cores, `FA_TEST_WORKERS`), reported in the order of the files; exits with 1
when a test fails. `--watch` runs them again
after each change. In a workspace, without a package, the tests of every package
with test files run. `--target=native` compiles them with the LLVM backend and runs
each test in its own run of the executable. `--coverage` is not available yet.
"
		}
		"develop" => {
			"\
Usage: fa develop [<package>] [--target=bun|web|native]

Compiles the application for Bun, runs it, watches the files of the package
(and of the packages of the workspace it uses), and reloads the edited modules
in the running program after each change (their module-level values initialized
again, `reactive` values and the others kept), or restarts it when the change
can't be reloaded (a type, the exports or the files changed;
`develop.hotReload = false` always restarts); a change with errors keeps the
running program until it is fixed. The test files that read an edited file run
again. `--target=web` serves the web build on `develop.port` and reloads the
modules in the open pages (or the pages); `--target=native` rebuilds the
executable (the unchanged units from the cache) and restarts it. In a
workspace, without a package, every package with an application is developed.
"
		}
		"preview" => {
			"\
Usage: fa preview [<package>] [--target=<target>]

Runs the output of `fa build`: without `--target`, the first target of the
configuration that runs here (native, then bun, node; web is served on
`develop.port`). The exit code is the program's.
"
		}
		"format" => {
			"\
Usage: fa format [<file.fa> | <folder>...] [--check]

Formats the source files: the given files, the `.fa` files of the given folders,
or, without a path, the closest package of the current folder (else its
workspace). `-`
formats the standard input. `--check` only lists the files that are not
formatted, and exits with 1 when there is one.
"
		}
		"update" => {
			"\
Usage: fa update [<dependency>...]

Resolves the dependencies again, to the latest versions their ranges allow, and
rewrites `dependencies.lock`: all of them, or the ones named (the keys of
`dependencies.fa`, or the source of an `@import`). Other commands never change a
locked version.
"
		}
		"new" => {
			"\
Usage: fa new package <name>
       fa new workspace [<name>]

`fa new package` creates the package of the hello-world guide in a new folder
<name>, with a test; the name must be an identifier (a letter or `_`, then
letters, digits or `_`). `fa new workspace` creates `workspace.fa`,
`dependencies.fa` and `packages/`, in a new folder <name> or in the current
folder.
"
		}
		"bind" => {
			"\
Usage: fa bind <header.h> [--output <file.foreign.fa>] [-- <C compiler flags>...]

Writes a foreign file declaring the functions, structs, enums and integer
`#define`s of a C header (to the standard output without `--output`). The
header is read without a C compiler: macros are not expanded, `#include`s are
not followed, and what can't be translated is written as a comment. The flags
after `--` are accepted and ignored for now.
"
		}
		"help" => "Usage: fa help [<command>]\n\nDescribes the commands, or one of them.\n",
		"version" => "Usage: fa version\n\nPrints the version of fa.\n",
		_ => return None,
	})
}
