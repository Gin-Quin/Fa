
import ./parsers/parseFaExpression
import ./ast/nodes/print

let ast = parseFaExpression("let y = 5\nif x == 12\n\trun(123)\nlet x = 11")

for node in ast.nodes:
  node.print()

# import ./readers/indentedContentReader

# var str = "Hello!\n\tSecond line\nHow are you?"
# var cstr = cstring(str)
# let ua = cast[ptr UncheckedArray[char]](cstr)

# let read = indentedContentReader(ua, str.len)

# for lineRange in read(0):
#   echo "start: ", lineRange.start, ", stop: ", lineRange.stop
