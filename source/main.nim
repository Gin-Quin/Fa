import ./parsers/fa
import ./ast/nodes/print

let ast = parseFa("++!y++++")

if ast != nil:
  ast.print()
