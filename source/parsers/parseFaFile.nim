import std/memfiles

import ./parseFa
import ./ParserData

proc parseFaFile*(fileName: string): ref ParserData =
  let memFile = memfiles.open(fileName)
  return parseFa(cast[ptr UncheckedArray[char]](memFile.mem), memFile.size)
