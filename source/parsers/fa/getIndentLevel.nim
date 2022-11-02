proc getIndentLevel*(line: cstring): int =
  while line[result] == '\t':
    result += 1

