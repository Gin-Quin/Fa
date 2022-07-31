import ./parsers/fa
import ./ast/nodes/print

let ast = parseFa("let x = 12 + 131 ** 2 * 4 + 3")

ast.print()
