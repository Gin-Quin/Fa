import ../readers/Reader
import ../ast/nodes/FaNode

type ParserData* = ref object
  data*: ptr UncheckedArray[char]
  length*: int
  reader*: Reader
  ast*: seq[FaNode]
