import ./parsers/fa
import ./ast/nodes/print

let ast = parseFa("let x = 12")

ast.print()
