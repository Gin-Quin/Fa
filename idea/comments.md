# Comments syntax

Let's discuss about the syntax of comments.

There are three nice ways of doing comments:

1. C-style:
- `//` for inline comments
- `/* */` for block comments

2. Python-style:
- `#` for inline comments
- `""" """` for block comments

3. Lua-style:
- `--` for inline comments
- `--[[` and `--]]` for block comments


Thoughts:

- we cannot use `#` because we want to use it for the hash symbol
- we'd like to use `//` for integer division
- I don't see any use case for the `--` syntax
- And I think `--` is the most readable syntax for inline comments

Proposal:

1. `--` for inline comments, `//` for integer division
2. `---` for block commments
3. Any comment placed before a declaration is a documentation comment (no special syntax for documentation)
4. If you want a comment to not be considered as a documentation comment, you can just add a newline before the declaration
