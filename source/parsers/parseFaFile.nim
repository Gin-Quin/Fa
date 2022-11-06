import std/memfiles

import ./parseFa

proc parseFaFile*(fileName: string): ParserData =
  let memFile = memfiles.open(fileName)
  return parseFa(cast[ptr UncheckedArray[char]](memFile.mem), memFile.size)
