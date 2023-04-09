
import ./parsers/parseFaFile
import ./ast/nodes/print

let ast = parseFaFile("source/sample.coffee")

for node in ast.nodes:
  node.print()

# import ./readers/indentedContentReader

# var str = "Hello!\n\tSecond line\nHow are you?"
# var cstr = cstring(str)
# let ua = cast[ptr UncheckedArray[char]](cstr)

# let read = indentedContentReader(ua, str.len)

# for lineRange in read(0):
#   echo "start: ", lineRange.start, ", stop: ", lineRange.stop
