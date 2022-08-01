import ./parsers/fa
import ./ast/nodes/print

let ast = parseFa("object.prop(12)")

ast.print()
