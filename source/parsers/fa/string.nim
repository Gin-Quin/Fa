import std/strutils

type
  Line = ref object
    slice*: string # need to convert this into an OpenArray
    subLines*: seq[Line]

proc getIndentLevel(line: string): int =
  while line[result] == '\t':
    result += 1

proc parseFaString*(value: string): seq[Line] =
  # let file = memfiles.open(filePath)
  var currentLevel = 0
  var stack = @[result]
  
  for slice in splitLines(value):
    let level = getIndentLevel(slice)
    let line = Line(slice: slice)

    if level == currentLevel:
      stack[^1].add(line)
    elif level == currentLevel + 1:
      stack.add(stack[^1][^1].subLines)
      stack[^1].add(line)
      currentLevel += 1
    elif level > currentLevel + 1:
      raise newException(IOError, "Forbidden double indentation")
    else:
      while level < currentLevel:
        currentLevel -= 1
        discard stack.pop()
