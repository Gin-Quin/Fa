import ./parsers/fa
import ./ast/nodes/print

let ast = parseFa("3 * add(\"hello\", 3) ** 5")

ast.print()
