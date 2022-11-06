import ./parseFa

proc parseFaExpresssion*(expression: string): ParserData =
  return parseFa(cast[ptr UncheckedArray[char]](cstring(expression)), expression.len)
