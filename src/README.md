The Fa compiler is a set of modules with different purposes :

- The `lexer` scans a Fa source file and create a list of tokens.
- The `parser` takes this list of tokens as input and output a full AST.
- The AST is then handled by generators. A `generator` produces an output from an AST or directly interpret the AST.
- Finally, the `bundler` takes care of resolving dependencies.
