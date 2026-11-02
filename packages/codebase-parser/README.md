# Codebase Parser

The files of a Fa codebase and the name tables of its folders (advanced brief §15.2): `Codebase::scan_package` and `Codebase::scan_standalone` read the `.fa` files of a package or of the folder of a standalone file, `Codebase::from_paths` builds the same from relative paths. Each folder gets a table of its entries (files by their name, folders as namespaces, the contents of `(group)` folders merged), and `Codebase::lookup` resolves a name from a file: its siblings, then its root's entries, then the `library` namespace from `application/` and `scripts/`. Invalid names (`MOD001`), names that differ only by case (`MOD002`) and two entries with the same name (`MOD003`) are reported on the entries.

Nothing is parsed here: the analyzer reads the files through its `ModuleResolver` (`fa_analyzer::CodebaseResolver`).
