import ./parseFa
import ./ParserData

proc parseFaExpresssion*(expression: string): ref ParserData =
  return parseFa(cast[ptr UncheckedArray[char]](cstring(expression)), expression.len)
