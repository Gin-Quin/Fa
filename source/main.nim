import ./parsers/fa
import ./ast/nodes/print

let ast = parseFa("run x + y")

ast.print()
