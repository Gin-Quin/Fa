import ../types/Range

proc indentedContentReader*(value: ptr UncheckedArray[char], length: int): IndentedContentReader =
  # let file = memfiles.open(filePath)
  var currentLevel = 0
  var start = 0
  var stop = 0
  var isEmpty = true

  proc getCurrentLevel() =
    currentLevel = 0
    while value[start + currentLevel] == '\t':
      currentLevel += 1
  
  proc getNextLine() =
    isEmpty = true
    start = stop
    while value[stop] != '\n' and value[stop] != '\r' and stop < length:
      if value[stop] != '\t' and value[stop] != ' ':
        isEmpty = false
      stop += 1
    if stop == length:
      stop = -1
    elif isEmpty or stop == start:
      stop += 1
      getNextLine()

  getNextLine()
  getCurrentLevel()

  return iterator(indentLevel = 0): Range =
    while true:
      if currentLevel < indentLevel:
        return

      if stop == -1:
        if isEmpty == false:
          result = Range(start: start, stop: length)
          yield result # we yield the last value
        return # and then return
      else:
        result = Range(start: start, stop: stop)
        getNextLine()
        getCurrentLevel()
        yield result
