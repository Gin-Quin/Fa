import ../readers/Reader
import ../ast/nodes

type ParserData* = ref object
  data*: ptr UncheckedArray[char]
  length*: int
  reader*: Reader
  nodes*: seq[FaNode]
