import ./parsers/fa
import ./ast/nodes/print

let ast = parseFa("if x == 12\n\trun 123\n\tlet y = 5\nlet x = 11")

for node in ast:
  node.print()
